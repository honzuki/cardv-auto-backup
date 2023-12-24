use egui::{Color32, RichText};

use crate::tg::Bot;

#[derive(Debug)]
pub struct SignIn {
    token: String,
    channel_id: String,
    state: SignInState,
}

#[derive(Debug)]
enum SignInState {
    EnterDetails,
    AttemptLogin,
}

impl SignIn {
    pub fn new() -> Self {
        Self {
            token: Default::default(),
            channel_id: Default::default(),
            state: SignInState::EnterDetails,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) -> Option<Bot> {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Connect a new bot");

                let token_label = ui.label("Bot token: ");
                ui.text_edit_singleline(&mut self.token)
                    .labelled_by(token_label.id);

                let channel_id_label = ui.label("Target channel id: ");
                ui.text_edit_singleline(&mut self.channel_id)
                    .labelled_by(channel_id_label.id);

                let channel_id = self.channel_id.parse::<i64>();

                let (status, color) = match channel_id {
                    Ok(_) if !self.token.is_empty() => (true, Color32::GREEN),
                    _ => (false, Color32::RED),
                };

                if ui
                    .add_enabled(
                        status,
                        egui::Button::new(RichText::new("connect").color(color).size(16.0)),
                    )
                    .clicked()
                {
                    let channel_id = channel_id.expect("only clickable when channel id is valid");
                    tracing::info!("{}", channel_id);
                }

                ui.label(format!(
                    "token: {}, channel_id: {:?}",
                    self.token,
                    self.channel_id.parse::<i64>()
                ));
            });
        });

        None
    }
}
