use gtk4::prelude::*;
use gtk4::{Application, glib};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::runtime::Runtime;

mod ai_service;
mod clipboard;
mod popup;
mod settings;
mod settings_window;
mod tray;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static HOTKEY_PAUSED: OnceLock<Arc<std::sync::atomic::AtomicBool>> = OnceLock::new();

pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("tokio runtime"))
}

pub fn hotkey_paused() -> &'static Arc<std::sync::atomic::AtomicBool> {
    HOTKEY_PAUSED.get_or_init(|| Arc::new(std::sync::atomic::AtomicBool::new(false)))
}

fn main() {
    let app = Application::builder()
        .application_id("dev.lingcloud.aihelper")
        .build();

    let initialized = Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.connect_activate(move |app| {
        if initialized.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return; // Already initialized, ignore re-activation
        }
        Box::leak(Box::new(app.hold()));
        
        let settings = Arc::new(Mutex::new(settings::Settings::load()));
        tray::setup_tray(app, settings.clone());
        setup_global_shortcut(app.clone(), settings);
    });

    app.run();
}

fn setup_global_shortcut(app: Application, settings: Arc<Mutex<settings::Settings>>) {
    // Register GNOME custom keybinding that sends a D-Bus signal to us
    register_gnome_shortcut(&settings.lock().unwrap().hotkey);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let paused = hotkey_paused().clone();

    runtime().spawn({
        let paused = paused.clone();
        async move {
            if let Err(e) = listen_dbus_trigger(tx, paused).await {
                eprintln!("[hotkey] D-Bus listener error: {e:#?}");
            }
        }
    });

    glib::spawn_future_local(async move {
        while rx.recv().await.is_some() {
            trigger_popup(&app, settings.clone());
        }
    });
}

pub fn register_gnome_shortcut(hotkey: &str) {
    use std::process::Command;

    let hotkey = if hotkey.is_empty() { "<Ctrl><Shift>g" } else { hotkey };

    let binding_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/aihelper/";
    let list_key = "org.gnome.settings-daemon.plugins.media-keys";

    // Get current custom keybindings list
    let output = Command::new("gsettings")
        .args(["get", list_key, "custom-keybindings"])
        .output()
        .ok();
    let current = output.as_ref().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();

    // Add our binding to the list if not already there
    let new_list = if current == "@as []" || current == "[]" {
        format!("['{}']", binding_path)
    } else if current.contains(binding_path) {
        current.to_string()
    } else {
        // Insert into existing list
        current.replacen('[', &format!("['{}', ", binding_path), 1)
    };

    let _ = Command::new("gsettings").args(["set", list_key, "custom-keybindings", &new_list]).status();

    let schema = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
    let _ = Command::new("gsettings").args(["set", &format!("{}:{}", schema, binding_path), "name", "AIHelper Trigger"]).status();
    let _ = Command::new("gsettings").args(["set", &format!("{}:{}", schema, binding_path), "command",
        "busctl --user call dev.lingcloud.aihelper.hotkey /trigger dev.lingcloud.Trigger Fire"
    ]).status();
    let _ = Command::new("gsettings").args(["set", &format!("{}:{}", schema, binding_path), "binding", hotkey]).status();
}

async fn listen_dbus_trigger(tx: tokio::sync::mpsc::UnboundedSender<()>, paused: Arc<std::sync::atomic::AtomicBool>) -> Result<(), zbus::Error> {
    use zbus::{connection, interface};

    struct TriggerIface {
        tx: tokio::sync::mpsc::UnboundedSender<()>,
        paused: Arc<std::sync::atomic::AtomicBool>,
    }

    #[interface(name = "dev.lingcloud.Trigger")]
    impl TriggerIface {
        fn fire(&self) {
            if !self.paused.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = self.tx.send(());
            }
        }
    }

    let _conn = connection::Builder::session()?
        .name("dev.lingcloud.aihelper.hotkey")?
        .serve_at("/trigger", TriggerIface { tx, paused })?
        .build()
        .await?;

    std::future::pending::<()>().await;
    Ok(())
}

fn trigger_popup(app: &Application, settings: Arc<Mutex<settings::Settings>>) {
    let app = app.clone();
    // Small delay to let the selection settle after the hotkey press
    glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
        let text = clipboard::read_clipboard();
        popup::show_popup(&app, text, settings);
    });
}
