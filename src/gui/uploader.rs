use crate::tg::Bot;

#[derive(Debug)]
pub struct Uploader {
    bot: Bot,
    state: UploaderState,
}

#[derive(Debug)]
enum UploaderState {
    BeforeStart,
    Uploading,
    Finished,
}

impl Uploader {
    pub fn new(bot: Bot) -> Self {
        Self {
            bot,
            state: UploaderState::BeforeStart,
        }
    }
}
