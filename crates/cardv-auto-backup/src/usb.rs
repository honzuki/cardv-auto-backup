use egui::TextBuffer;
use std::{collections::HashSet, fs, io, path::PathBuf, time::Duration};
use tokio::sync::mpsc as tokio_mpsc;

use crate::tg::{Bot, BotErr};

/// Waits until an sd-card containing 'CARDV' is inserted into the computer
///
/// returns the path to the base folder containing all the records
///
/// Notes:
/// this function uses long-polling by querying the OS
/// every interval lapse to see if a new device was plugged.
pub fn wait_for_cardv_drive() -> PathBuf {
    let mut checked: HashSet<char> = Default::default();

    loop {
        // we need to re-initialize the checked set
        // every time to not miss when a device is swapped.
        let mut new_checked: HashSet<char> = Default::default();
        for drive in usb::list_all_logical_drives() {
            new_checked.insert(drive);

            if checked.contains(&drive) {
                continue;
            }

            let path = PathBuf::from(format!("{}:\\CARDV\\Movie", drive));
            if path.exists() {
                return path;
            }
        }
        checked = new_checked;

        // thanks to the hashing we can check it frequently without
        // putting load on the system
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[derive(Debug)]
pub struct DriveUploader {
    rx: tokio_mpsc::UnboundedReceiver<UploaderMsg>,
}

#[derive(Debug)]
pub enum UploaderMsg {
    BadFileSystem,
    Interrupted(BotErr),
    Start(usize),
    Update(Update),
    // we need a seperate update for when a file was
    // uploaded successfully so we can save it to storage
    Uploaded(PathBuf),
    Done,
}

#[derive(Debug)]
pub struct Update {
    pub uploading: String,
    pub current: usize,
}

impl DriveUploader {
    pub fn new(bot: Bot, last_uploaded: Option<String>) -> Self {
        let (tx, rx) = tokio_mpsc::unbounded_channel();
        tokio::spawn(async move {
            let res = tokio::task::block_in_place(|| {
                let base = wait_for_cardv_drive();
                let mut files = fs::read_dir(base)?
                    .map(|path| path.map(|path| path.path()))
                    .filter(|path| match path {
                        Ok(path) => path
                            .to_string_lossy()
                            .trim()
                            .to_lowercase()
                            .ends_with("mp4"),
                        Err(_) => true,
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                files.sort(); // alpehetical ordering
                let total = files.len();

                // skip up to last-uploaded
                if let Some(last_uploaded) = last_uploaded {
                    // if 'last_uploaded' isn't part of the files we
                    // should not skip anything because the dates may clamp
                    if files
                        .iter()
                        .any(|file| file.to_string_lossy() == last_uploaded.as_str())
                    {
                        files.retain(|file| {
                            file.to_string_lossy().as_str() > last_uploaded.as_str()
                        });
                    }
                }
                let skip = total - files.len();

                Ok::<_, io::Error>((files, skip))
            });

            let Ok((files, skip)) = res else {
                let _ = tx.send(UploaderMsg::BadFileSystem);
                return;
            };

            if let Err(err) = drive_upload_worker(bot, files, skip, tx.clone()).await {
                tracing::error!("the upload has been failed: {err}");
                let _ = tx.send(UploaderMsg::Interrupted(err));
            }
        });

        Self { rx }
    }

    /// Pull a msg if there is any
    pub fn try_recv(&mut self) -> Option<UploaderMsg> {
        match self.rx.try_recv() {
            Ok(msg) => Some(msg),
            Err(_) => None,
        }
    }
}

#[tracing::instrument(skip(files))]
async fn drive_upload_worker(
    mut bot: Bot,
    files: Vec<PathBuf>,
    skip: usize,
    tx: tokio_mpsc::UnboundedSender<UploaderMsg>,
) -> Result<(), BotErr> {
    let _ = tx.send(UploaderMsg::Start(files.len() + skip));

    for (idx, file) in files.into_iter().enumerate() {
        if tx
            .send(UploaderMsg::Update(Update {
                uploading: file.file_name().unwrap().to_string_lossy().to_string(),
                current: idx + skip,
            }))
            .is_err()
        {
            tracing::info!("early termination of upload worker because listener was dropped");
            return Ok(());
        }

        // retry every increasing interval until successfull upload
        let mut interval = 60 * 10;
        loop {
            // Wait a bit to avoid hiting rate limits
            tokio::time::sleep(Duration::from_secs(30)).await;
            // re-login to reset connection issues
            bot = Bot::from_packed(bot.packed()).await?;

            if let Ok(res) =
                tokio::time::timeout(Duration::from_secs(interval), bot.upload_mp4(file.clone()))
                    .await
            {
                res?;
                break;
            }

            interval += 60;
        }
        let _ = tx.send(UploaderMsg::Uploaded(file));
    }

    let _ = tx.send(UploaderMsg::Done);

    Ok(())
}
