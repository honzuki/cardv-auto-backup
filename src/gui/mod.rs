use crate::tg::Bot;

mod login;
mod uploader;

use login::SignIn;
use uploader::Uploader;

#[derive(Debug)]
pub struct App {
    // bot: Option<Bot>,
    // token: String,
    // channel_id: String,
    state: State,
}

#[derive(Debug)]
enum State {
    SignIn(SignIn),
    Uploader(Uploader),
}

impl App {
    pub fn new() -> Self {
        Self {
            state: State::SignIn(SignIn::new()),
        }
    }

    pub fn new_with_bot(bot: Bot) -> Self {
        Self {
            state: State::Uploader(Uploader::new(bot)),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(2.0);

        match &mut self.state {
            State::SignIn(sign_in) => {
                sign_in.show(ctx, frame);
            }
            _ => {}
        }
    }
}
