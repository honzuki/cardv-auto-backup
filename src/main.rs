#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::time::Duration;

use eframe::egui;
use tg::{Bot, PackedBot};

mod gui;
mod tg;

use gui::App;

const PACKED_BOT_STORAGE_KEY: &str = "PACKED_BOT";

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _enter = runtime.enter();

    let async_handle = tokio::runtime::Handle::current();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 450.0])
            .with_app_id("CARDV_AUTO_BACKUP"),
        centered: true,
        follow_system_theme: true,
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "CARDV Backup",
        options,
        Box::new(|ctx| {
            // extract last used bot if exists
            let bot = ctx
                .storage
                .and_then(|storage| storage.get_string(PACKED_BOT_STORAGE_KEY))
                .and_then(|packed_bot| serde_json::from_str::<PackedBot>(&packed_bot).ok())
                .and_then(move |packed_bot| {
                    async_handle.block_on(async {
                        match Bot::from_packed(packed_bot).await {
                            Ok(bot) => Some(bot),
                            Err(err) => {
                                tracing::error!(?err);
                                None
                            }
                        }
                    })
                });

            let app = match bot {
                Some(bot) => App::new_with_bot(bot),
                None => App::new(),
            };

            Box::new(app)
        }),
    ) {
        tracing::error!(?err, "egui has crashed");
    }

    runtime.shutdown_timeout(Duration::from_secs(60));
    Ok(())
}
