use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum AIAction {
    FixSpelling,
    FollowUp,
    TranslateVI,
    TranslateKO,
}

impl AIAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FixSpelling => "Fix Spelling & Grammar",
            Self::FollowUp => "Follow-up",
            Self::TranslateVI => "Dịch sang Tiếng Việt",
            Self::TranslateKO => "Dịch sang Tiếng Hàn",
        }
    }

    fn system_prompt(&self) -> &'static str {
        match self {
            Self::FixSpelling => {
                "SYSTEM: You are a robotic grammar correction tool.\n\
                 RULES:\n\
                 - Provide ONLY the corrected text.\n\
                 - NO preamble.\n\
                 - NO explanation.\n\
                 - NO alternatives.\n\
                 - If the input is a fragment, complete it naturally.\n\
                 - Return exactly one string."
            }
            Self::FollowUp => {
                "You are a helpful and intelligent assistant. Answer the user's question \
                 or follow-up request accurately based on the provided text context. Be detailed yet concise."
            }
            Self::TranslateVI => {
                "Translate the following text to natural Vietnamese. Return ONLY the translation. No preamble."
            }
            Self::TranslateKO => {
                "Translate the following text to natural Korean. Return ONLY the translation. No preamble."
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Token {
    Thinking(String),
    Content(String),
}

pub async fn stream_ai(
    action: AIAction,
    text: String,
    base_url: String,
    model: String,
    api_key: String,
    enable_thinking: bool,
    tx: mpsc::UnboundedSender<Result<Token, String>>,
) {
    let url = format!(
        "{}/chat/completions",
        base_url.trim_end_matches('/')
    );

    let mut body = json!({
        "model": model,
        "stream": true,
        "messages": [
            {"role": "system", "content": action.system_prompt()},
            {"role": "user",   "content": text}
        ]
    });

    if enable_thinking && base_url.contains("11434") {
        body["think"] = json!(true);
    }

    let client = Client::new();
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("User-Agent", "Suano/1.0");

    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = match req.json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(Err(e.to_string()));
            return;
        }
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        let msg = extract_api_error(&body_text)
            .unwrap_or_else(|| format!("HTTP {status}: {body_text}"));
        let _ = tx.send(Err(msg));
        return;
    }

    let mut stream = resp.bytes_stream();
    let mut in_think = false;
    let mut buf = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };

        buf.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE lines
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();

            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];
            if data == "[DONE]" {
                return;
            }

            let Ok(json): Result<Value, _> = serde_json::from_str(data) else {
                continue;
            };

            let delta = json
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"));

            let Some(delta) = delta else { continue };

            // Explicit reasoning fields
            if let Some(reasoning) = delta
                .get("reasoning_content")
                .or_else(|| delta.get("thinking"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                let _ = tx.send(Ok(Token::Thinking(reasoning.to_string())));
                continue;
            }

            let Some(content) = delta.get("content").and_then(|v| v.as_str()) else {
                continue;
            };
            if content.is_empty() {
                continue;
            }

            // Parse inline <think> tags
            let mut remaining = content;
            while !remaining.is_empty() {
                if !in_think {
                    if let Some(idx) = remaining.find("<think>") {
                        if idx > 0 {
                            let _ = tx.send(Ok(Token::Content(remaining[..idx].to_string())));
                        }
                        in_think = true;
                        remaining = &remaining[idx + 7..];
                    } else {
                        let _ = tx.send(Ok(Token::Content(remaining.to_string())));
                        break;
                    }
                } else if let Some(idx) = remaining.find("</think>") {
                    if idx > 0 {
                        let _ = tx.send(Ok(Token::Thinking(remaining[..idx].to_string())));
                    }
                    in_think = false;
                    remaining = &remaining[idx + 8..];
                } else {
                    let _ = tx.send(Ok(Token::Thinking(remaining.to_string())));
                    break;
                }
            }
        }
    }
}

pub async fn fetch_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = Client::new();
    let mut req = client.get(&url).header("Content-Type", "application/json");
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let models = json
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            let mut ids: Vec<String> = arr
                .iter()
                .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(String::from))
                .collect();
            ids.sort();
            ids
        })
        .unwrap_or_default();
    Ok(models)
}

fn extract_api_error(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    json.get("error")?
        .get("message")?
        .as_str()
        .map(String::from)
}
