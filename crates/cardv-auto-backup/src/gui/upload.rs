use eframe::Storage;
use egui::{Color32, ProgressBar, RichText, Spinner};

use crate::{
    tg::Bot,
    usb::{DriveUploader, UploaderMsg},
};

const LAST_UPLOAD_STORAGE_KEY: &str = "LAST_UPLOAD";

#[derive(Debug)]
pub struct Uploader {
    uploader: DriveUploader,
    state: State,
}

#[derive(Debug)]
enum State {
    Error(String),
    WaitForDrive,
    Uploading(UploadingState),
    Finished,
}

#[derive(Debug)]
struct UploadingState {
    current_name: Option<String>,
    current: usize,
    total: usize,
}

impl UploadingState {
    fn show(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(format!("Uploading {} files", self.total));
                ui.spinner();

                let text = match &self.current_name {
                    Some(name) => format!("{} - {}/{}", name, self.current, self.total),
                    None => format!("{}/{}", self.current, self.total),
                };

                ui.add(
                    ProgressBar::new((self.current as f32) / (self.total as f32))
                        .text(text)
                        .fill(Color32::GREEN),
                );
            });
        });
    }
}

impl Uploader {
    pub fn new(bot: Bot, storage: Option<&dyn Storage>) -> Self {
        Self {
            uploader: DriveUploader::new(
                bot,
                storage.and_then(|storage| storage.get_string(LAST_UPLOAD_STORAGE_KEY)),
            ),
            state: State::WaitForDrive,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match &self.state {
            State::WaitForDrive => self.wait_for_drive(ctx),
            State::Uploading(uploading) => uploading.show(ctx),
            State::Error(reason) => Self::error(reason, ctx),
            State::Finished => Self::finished(ctx),
        }

        if let Some(msg) = self.uploader.try_recv() {
            match (&mut self.state, msg) {
                (State::WaitForDrive, UploaderMsg::Start(total)) => {
                    self.state = State::Uploading(UploadingState {
                        current_name: None,
                        current: 0,
                        total,
                    })
                }
                (State::Uploading(uploading), UploaderMsg::Update(update)) => {
                    uploading.current_name = Some(update.uploading);
                    uploading.current = update.current;
                }
                (State::Uploading(_), UploaderMsg::Uploaded(path)) => {
                    if let Some(storage) = frame.storage_mut() {
                        storage.set_string(
                            LAST_UPLOAD_STORAGE_KEY,
                            path.to_string_lossy().to_string(),
                        );
                    }
                }
                (State::Uploading(_), UploaderMsg::Done) => self.state = State::Finished,
                (_, UploaderMsg::BadFileSystem) => {
                    self.state = State::Error("failed to read the filesystem".into());
                }
                (_, UploaderMsg::Interrupted(reason)) => {
                    self.state = State::Error(format!("uploader was interrupted: {reason}"));
                }
                _ => self.state = State::Error("unexpected error!".into()),
            }
        }
    }

    fn wait_for_drive(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Please insert the device");
                ui.add(Spinner::new().size(30.0));
            });
        });
    }

    fn error(reason: &str, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new("The backup was failed!").color(Color32::RED));
                ui.label(RichText::new(format!("reason: {}", reason)).monospace());
            });
        });
    }

    fn finished(ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(
                    RichText::new("The backup has been completed successfully!")
                        .color(Color32::GREEN),
                );
                ui.label(
                    RichText::new("you can safely remove the device through the windows interface")
                        .strong(),
                );
            });
        });
    }
}
