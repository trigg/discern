use crate::data::ConnState;
use futures::lock::Mutex;
use std::sync::Arc;
use tokio::net::TcpStream;
extern crate serde_json;
use futures_util::stream::SplitSink;
use serde_json::Value;
use std::fs;
use string_builder::Builder;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub fn start(
    file_path: String,
    _writer: Arc<
        Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>>,
    >,
    _matches: &clap::ArgMatches,
    gui_state: Arc<Mutex<ConnState>>,
) -> impl Fn(String) {
    // We're not doing a thread for output
    //let _gui_loop = tokio::task::spawn(async move {
    //    loop {
    //        sleep(Duration::from_millis(1000)).await;
    //    }
    //});

    move |value| {
        // clone all data and spawn an async task
        let gui_state = gui_state.clone();
        let value = value.clone();
        let file_path = file_path.clone();

        tokio::task::spawn(async move {
            let data: Value = serde_json::from_str(&value).unwrap();
            match data["cmd"].as_str() {
                Some(_) => {
                    let state = gui_state.lock().await.clone();
                    if state.user_id.is_some() && state.voice_channel.is_some() {
                        let mut builder = Builder::default();
                        builder.append(state.voice_channel.unwrap());
                        builder.append("\n");
                        builder.append(format!("{}", state.users.len()));
                        builder.append("\n");
                        for (id, user) in state.users {
                            match state.voice_states.get(&id) {
                                Some(voice_state) => {
                                    match &voice_state.nick {
                                        Some(nick) => builder.append(nick.clone()),
                                        None => builder.append(user.username),
                                    }
                                    builder.append("\n");
                                    if voice_state.mute || voice_state.self_mute {
                                        builder.append("m");
                                    } else {
                                        builder.append(".");
                                    }
                                    if voice_state.deaf || voice_state.self_deaf {
                                        builder.append("d");
                                    } else {
                                        builder.append(".");
                                    }
                                    if voice_state.talking {
                                        builder.append("t");
                                    } else {
                                        builder.append(".");
                                    }
                                    builder.append("\n");
                                    match user.avatar {
                                        Some(avatar) => {
                                            builder.append(format!(
                                                "https://cdn.discordapp.com/avatars/{}/{}.png",
                                                user.id, avatar
                                            ));
                                        }
                                        None => {}
                                    }

                                    builder.append("\n");
                                }
                                None => {}
                            }
                        }
                        fs::write(file_path, builder.string().unwrap())
                            .expect("Unable to write statefile");
                    } else {
                        // 0 Means no channel - and therefore no further data
                        fs::write(file_path, "0\n").expect("Unable to write statefile");
                    }
                } // Every message may be a change of state. Too often?
                None => {}
            }
        });
    }
}
