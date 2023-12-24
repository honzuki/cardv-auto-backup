use std::{fmt::Debug, path::Path, time::Duration};

use grammers_client::{
    types::{Attribute, PackedChat},
    Client, Config, InputMessage, Update,
};
use grammers_session::Session;

const API_ID: i32 = 6;
const API_HASH: &str = "eb06d4abfb49dc3eeb1aeb98ae0f581e";

const IDENTIFY_MESSAGE: &str = "this-message-is-used-by-the-bot-to-get-the-channel-hash";

#[derive(thiserror::Error, Debug)]
pub enum BotErr {
    #[error("failed to connect to telegram servers")]
    Communication,

    #[error("failed to complete the request: {0}")]
    Invocation(#[from] grammers_mtsender::InvocationError),

    #[error("failed to load bot from packed version because the session is corrupted")]
    BadSession(#[from] grammers_session::Error),

    #[error("failed to authorize the bot ({0})")]
    BadAuth(#[from] grammers_client::client::updates::AuthorizationError),

    #[error("the target chat was corrupted")]
    CorruptedTargetChat,

    #[error("failed to find the target chat")]
    NoTargetChat,

    #[error("{0}")]
    Io(#[from] tokio::io::Error),

    #[error("failed to extract the video attribute from path")]
    NoVideoAttribute,

    #[error("failed to create a thumbnail from the video")]
    NoThumbnail,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PackedBot {
    session: Vec<u8>,
    target_chat: Vec<u8>,
}

#[derive(Debug)]
pub struct Bot {
    client: Client,
    target_channel: PackedChat,
}

impl Bot {
    /// Create a new bot session
    ///
    /// this bot can later be packed and saved to a file
    /// to avoid needing to authenticate every single time
    #[tracing::instrument]
    pub async fn new(token: &str, target_channel: i64) -> Result<Self, BotErr> {
        let client = Client::connect(Config {
            session: Session::new(),
            api_id: API_ID,
            api_hash: API_HASH.into(),
            params: Default::default(),
        })
        .await
        .map_err(|_| BotErr::Communication)?;

        let user = client.bot_sign_in(token).await?;
        tracing::info!("logged in as: {}", user.full_name());

        let target_channel = find_channel(&client, token, target_channel).await?;

        Ok(Self {
            client,
            target_channel,
        })
    }

    /// Try to load the bot from its packed version
    #[tracing::instrument]
    pub async fn from_packed(packed: PackedBot) -> Result<Self, BotErr> {
        let target_channel =
            PackedChat::from_bytes(&packed.target_chat).map_err(|_| BotErr::CorruptedTargetChat)?;
        let session = Session::load(&packed.session)?;

        let client = Client::connect(Config {
            session,
            api_id: API_ID,
            api_hash: API_HASH.into(),
            params: Default::default(),
        })
        .await?;

        // we'll never pack an unauthorized bot
        debug_assert!(client.is_authorized().await?);

        Ok(Self {
            client,
            target_channel,
        })
    }

    /// Pack the bot into a serializable structure
    pub fn packed(&self) -> PackedBot {
        PackedBot {
            session: self.client.session().save(),
            target_chat: self.target_channel.to_bytes().into(),
        }
    }

    /// Uploads an mp4 video to the target channel
    pub async fn upload_mp4(
        &self,
        path: impl AsRef<Path> + Debug + Clone + Send + 'static,
    ) -> Result<(), BotErr> {
        let attribute = get_mp4_attribute(path.clone()).await?;
        let video = self.client.upload_file(path.clone()).await?;

        self.client
            .send_message(
                self.target_channel,
                InputMessage::default()
                    .mime_type("video/mp4")
                    .document(video)
                    .attribute(attribute),
            )
            .await?;

        Ok(())
    }
}

#[tracing::instrument]
async fn find_channel(client: &Client, token: &str, channel: i64) -> Result<PackedChat, BotErr> {
    // The bot can't find the chat has from its chat_id, so we need to use
    // the botapi to send a message by chat_id and wait for the bot to receive
    // this update, which contains the access_hash and can be saved for future use.
    //
    // this is a dirty work around, but there does not seem to be an easier way
    reqwest::get(format!(
        "https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}",
        token, channel, IDENTIFY_MESSAGE,
    ))
    .await
    .map_err(|err| {
        tracing::error!(?err);
        BotErr::Communication
    })?;

    let chat = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(update) = client.next_update().await? {
            if let Update::NewMessage(message) = update {
                if message.outgoing() && message.text() == IDENTIFY_MESSAGE {
                    message.delete().await?;
                    tracing::info!("found target_chat: {}", message.chat().name());
                    return Ok(Some(message.chat().pack()));
                }
            }
        }
        Ok::<_, grammers_mtsender::InvocationError>(None)
    })
    .await
    .map_err(|_| BotErr::NoTargetChat)??
    .ok_or(BotErr::NoTargetChat)?;

    Ok(chat)
}

#[tracing::instrument]
async fn get_mp4_attribute(
    path: impl AsRef<Path> + Debug + Send + 'static,
) -> Result<Attribute, BotErr> {
    use std::{fs, io};

    tokio::task::spawn_blocking(move || {
        let file = fs::File::open(path)?;
        let size = file.metadata()?.len();
        let reader = io::BufReader::new(file);

        let mp4 = mp4::Mp4Reader::read_header(reader, size).map_err(|err| {
            tracing::error!(?err);
            BotErr::NoVideoAttribute
        })?;
        let track = mp4
            .tracks()
            .values()
            .next()
            .ok_or(BotErr::NoVideoAttribute)?;

        Ok::<Attribute, BotErr>(Attribute::Video {
            round_message: false,
            supports_streaming: true,
            duration: track.duration(),
            w: track.width() as i32,
            h: track.height() as i32,
        })
    })
    .await
    .map_err(|_| BotErr::NoVideoAttribute)?
}
