# AIHelper — Linux (Rust + GTK4)

A Fedora Linux/Wayland port of the macOS AIHelper app. Select text in any app, press **Ctrl+Shift+G**, and get instant AI writing assistance.

## Features

- Global hotkey (default `Ctrl+Shift+G`) via XDG Desktop Portal `GlobalShortcuts`
- Streaming AI responses (OpenAI-compatible APIs + Ollama)
- Follow-up chat in the popup
- `<think>` / reasoning token display (collapsible)
- Translation buttons (Vietnamese / Korean) after grammar fix
- Read/Paste-back background results to source app via `wl-clipboard` and `ydotool`
- Settings persisted to `~/.config/aihelper/settings.json`

## Prerequisites

```bash
# Fedora
sudo dnf install gtk4-devel libsecret-devel glib2-devel dbus-devel wl-clipboard ydotool
```

Rust toolchain: https://rustup.rs

## Build & Install

```bash
./install.sh
```

Or manually:

```bash
cargo build --release
./target/release/aihelper
```

## Configuration

Click the app menu → **Settings** to set:

- AI Provider: **OpenAI** (any OpenAI-compatible endpoint, e.g. Groq) or **Ollama**
- Base URL, Model, API Key
- Global hotkey (GTK accelerator format, e.g. `<Ctrl><Shift>g`)
- Translation language buttons

## Notes

- Requires a **Wayland** session (specifically uses Wayland portals + utilities). On Fedora with GNOME, use the default Wayland session.
- `ydotool` the background daemon must be enabled. `install.sh` handles this for you automatically.
