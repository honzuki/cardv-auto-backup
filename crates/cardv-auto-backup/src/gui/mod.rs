use crate::tg::{Bot, PackedBot};

mod login;
mod upload;

use eframe::{CreationContext, Storage};
use egui::Spinner;
use login::SignIn;
use tokio::sync::oneshot;
use upload::Uploader;

#[derive(Debug)]
pub struct App {
    state: State,
}

#[derive(Debug)]
enum State {
    SignIn(SignIn),
    LoadBot(oneshot::Receiver<Option<Bot>>),
    Uploader(Uploader),
}

impl App {
    pub fn new(ctx: &CreationContext) -> Self {
        // extract last used bot if exists
        let bot = ctx
            .storage
            .and_then(|storage| storage.get_string(crate::PACKED_BOT_STORAGE_KEY))
            .and_then(|packed_bot| serde_json::from_str::<PackedBot>(&packed_bot).ok());

        let state = match bot {
            Some(packed_bot) => {
                let (tx, rx) = oneshot::channel();
                tokio::spawn(async move {
                    match Bot::from_packed(packed_bot).await {
                        Ok(bot) => tx.send(Some(bot)),
                        Err(err) => {
                            tracing::error!(?err);
                            tx.send(None)
                        }
                    }
                });

                State::LoadBot(rx)
            }
            None => State::SignIn(SignIn::new()),
        };

        Self { state }
    }

    pub fn new_with_bot(bot: Bot, storage: Option<&dyn Storage>) -> Self {
        Self {
            state: State::Uploader(Uploader::new(bot, storage)),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(2.0);

        match &mut self.state {
            State::LoadBot(rx) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.add(Spinner::new().size(30.0));
                    })
                });

                match rx.try_recv() {
                    Ok(Some(bot)) => *self = Self::new_with_bot(bot, frame.storage()),
                    Ok(None) | Err(oneshot::error::TryRecvError::Closed) => {
                        self.state = State::SignIn(SignIn::new())
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                }
            }
            State::SignIn(sign_in) => {
                if let Some(bot) = sign_in.show(ctx) {
                    if let Some(storage) = frame.storage_mut() {
                        storage.set_string(
                            crate::PACKED_BOT_STORAGE_KEY,
                            serde_json::to_string(&bot.packed())
                                .expect("serializing into string should never fail"),
                        )
                    }

                    self.state = State::Uploader(Uploader::new(bot, frame.storage()));
                }
            }
            State::Uploader(uploader) => uploader.show(ctx, frame),
        }
    }
}
