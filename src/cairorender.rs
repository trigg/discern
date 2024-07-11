use crate::data::ConnState;
use bytes::Bytes;
use futures::SinkExt;
use futures_util::StreamExt;
use std::collections::hash_map::HashMap;

#[derive(Debug, Clone, Hash)]
pub struct DiscordAvatarRaw {
    pub key: String,
    pub raw: Option<Bytes>,
}

pub async fn avatar_downloader(
    mut sender: futures::channel::mpsc::Sender<DiscordAvatarRaw>,
    mut recvr: futures::channel::mpsc::Receiver<ConnState>,
) {
    tokio::spawn(async move {
        println!("Starting avatar thread");
        let mut already_done: HashMap<String, Option<Bytes>> = HashMap::new();

        while let Some(state) = recvr.next().await {
            println!("Avatar State : {:?}", state);
            for (_key, value) in state.users.into_iter() {
                println!("Avatar Check : {:?}", value);
                if value.avatar.is_some() {
                    let avatar_key: String = format!("{}/{}", value.id, value.avatar.unwrap());
                    if !already_done.contains_key(&avatar_key) {
                        println!("Requesting {}", avatar_key);
                        let url = format!("https://cdn.discordapp.com/avatars/{}.png", avatar_key);
                        match reqwest::Client::new()
                            .get(url)
                            .header("Referer", "https://streamkit.discord.com/overlay/voice")
                            .header("User-Agent", "Mozilla/5.0")
                            .send()
                            .await
                        {
                            Ok(resp) => match resp.bytes().await {
                                Ok(bytes) => {
                                    already_done.insert(avatar_key.clone(), Some(bytes.clone()));
                                    match sender
                                        .send(DiscordAvatarRaw {
                                            key: avatar_key.clone(),
                                            raw: Some(bytes.clone()),
                                        })
                                        .await
                                    {
                                        Ok(_v) => {}
                                        Err(_e) => {}
                                    }
                                }
                                Err(_err) => {
                                    already_done.insert(avatar_key.clone(), None);
                                    match sender
                                        .send(DiscordAvatarRaw {
                                            key: avatar_key.clone(),
                                            raw: None,
                                        })
                                        .await
                                    {
                                        Ok(_v) => {}
                                        Err(_e) => {}
                                    }
                                }
                            },
                            Err(err) => {
                                println!("{}", err);
                            }
                        }
                    }
                }
            }
        }
        println!("AVATAR THREAD ENDED");
    });
}
