extern crate clap;
extern crate serde_json;
use data::ConnState;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use std::env;
use std::fs;
use std::sync::Arc;
use string_builder::Builder;

mod core;
mod data;
mod macros;

#[tokio::main]
async fn main() {
    let file_path = env::var("DISCERN_STATEFILE")
        .expect("No DISCERN_STATEFILE environment variable set. Quitting");

    // Websocket events to main thread
    let (event_sender, event_recv) = futures::channel::mpsc::channel::<ConnState>(10);
    let event_sender = Arc::new(Mutex::new(event_sender));
    let event_recv = Arc::new(Mutex::new(event_recv));

    // Main thread messages to Websocket output
    let (msg_sender, msg_recv) = futures::channel::mpsc::channel::<String>(10);
    let _msg_sender = Arc::new(Mutex::new(msg_sender));
    let msg_recv = Arc::new(Mutex::new(msg_recv));

    // Start a thread for connection
    let connector_event_sender = event_sender.clone();
    let connector_msg_recv = msg_recv.clone();
    core::connector(connector_event_sender.clone(), connector_msg_recv.clone()).await;

    loop {
        while let Some(state) = event_recv.lock().await.next().await {
            let file_path = file_path.clone();
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
                fs::write(file_path, builder.string().unwrap()).expect("Unable to write statefile");
            } else {
                // 0 Means no channel - and therefore no further data
                fs::write(file_path, "0\n").expect("Unable to write statefile");
            }
        }
    }
}
