use ksni::blocking::TrayMethods;
use tokio::sync::mpsc;
use gtk4::prelude::*;
use gtk4::{Application, glib};
use std::sync::{Arc, Mutex};

use crate::settings::Settings;

enum TrayCmd {
    Settings,
    Quit,
}

struct AppTray {
    tx: mpsc::UnboundedSender<TrayCmd>,
}

impl ksni::Tray for AppTray {
    fn icon_name(&self) -> String {
        "suano".into()
    }
    
    fn id(&self) -> String {
        "dev.lingcloud.suano".into()
    }

    fn title(&self) -> String {
        "Suano".into()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Settings".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCmd::Settings);
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCmd::Quit);
                }),
                ..Default::default()
            }.into(),
        ]
    }
}

pub fn setup_tray(app: &Application, settings: Arc<Mutex<Settings>>) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let tray = AppTray { tx };
    
    let handle = tray.spawn().expect("Failed to spawn tray");
    Box::leak(Box::new(handle)); // Keep tray handle alive forever

    let app = app.clone();
    glib::spawn_future_local(async move {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                TrayCmd::Settings => {
                    crate::settings_window::show_settings(&app, settings.clone(), || {});
                }
                TrayCmd::Quit => {
                    app.quit();
                }
            }
        }
    });
}
