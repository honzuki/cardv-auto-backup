#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::time::Duration;

use eframe::egui;

mod execution_state;
mod gui;
mod tg;
mod usb;

use gui::App;

const PACKED_BOT_STORAGE_KEY: &str = "PACKED_BOT";

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _enter = runtime.enter();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 350.0])
            .with_app_id("CARDV_AUTO_BACKUP"),
        centered: true,
        follow_system_theme: true,
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "CARDV Backup",
        options,
        Box::new(|ctx| Box::new(App::new(ctx))),
    ) {
        tracing::error!(?err, "egui has crashed");
    }

    runtime.shutdown_timeout(Duration::from_secs(60));
    Ok(())
}
