use egui::{Color32, RichText, Spinner};
use tokio::sync::oneshot;

use crate::tg::Bot;

#[derive(Debug)]
pub struct SignIn {
    token: String,
    channel_id: String,
    error: Option<String>,
    state: State,
}

#[derive(Debug)]
enum State {
    EnterDetails,
    AttemptLogin(oneshot::Receiver<Option<Bot>>),
}

impl SignIn {
    pub fn new() -> Self {
        Self {
            token: Default::default(),
            channel_id: Default::default(),
            error: None,
            state: State::EnterDetails,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<Bot> {
        let res = egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(matches!(self.state, State::EnterDetails));
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
                    let token = self.token.clone();
                    let channel_id = channel_id.expect("only clickable when channel id is valid");
                    let (tx, rx) = oneshot::channel();
                    tokio::spawn(async move {
                        match Bot::new(&token, channel_id).await {
                            Ok(bot) => {
                                let _ = tx.send(Some(bot));
                            }
                            Err(reason) => {
                                tracing::error!("failed to register: {reason}");
                                let _ = tx.send(None);
                            }
                        }
                    });

                    self.state = State::AttemptLogin(rx);
                }

                if let Some(err) = self.error.as_ref() {
                    ui.label(RichText::new(err).color(Color32::RED));
                }

                if let State::AttemptLogin(rx) = &mut self.state {
                    Spinner::new().size(20.0).paint_at(ui, ui.min_rect());

                    match rx.try_recv() {
                        Ok(Some(bot)) => return Some(bot),
                        Ok(None) => {
                            self.error = Some("failed to register the bot, please try again".into())
                        }
                        Err(oneshot::error::TryRecvError::Closed) => {
                            self.state = State::EnterDetails;
                        }
                        Err(oneshot::error::TryRecvError::Empty) => {}
                    };
                }

                None
            })
        });

        res.inner.inner
    }
}
