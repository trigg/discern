extern crate clap;
extern crate serde_json;
use futures::lock::Mutex;
use futures_util::{SinkExt, StreamExt};
use http::Request;
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::handshake::client::generate_key;

use crate::*;

async fn user_left_channel(state: Arc<Mutex<data::ConnState>>) {
    let mut current_state = state.lock().await;
    current_state.voice_channel = None;
    current_state.users.clear();
    current_state.voice_states.clear();
}

async fn set_user_talking(state: Arc<Mutex<data::ConnState>>, user_id: String, talking: bool) {
    let mut unlocked = state.lock().await;
    let mut voice_state = unlocked.voice_states.get_mut(&user_id).unwrap().clone();
    voice_state.talking = talking;
    unlocked.voice_states.insert(user_id.clone(), voice_state);
}

async fn update_state_from_voice_state(state: Arc<Mutex<data::ConnState>>, voice_state: &Value) {
    let user_id: String = voice_state["user"]["id"].as_str().unwrap().to_string();
    let mut current_state = state.lock().await;

    let username = voice_state["user"]["username"]
        .as_str()
        .unwrap()
        .to_string();
    let mut avatar: Option<String> = None;
    match voice_state["user"]["avatar"].as_str() {
        Some(in_avatar) => {
            avatar = Some(in_avatar.to_string());
        }
        None => {}
    }

    let user = data::DiscordUserData {
        avatar: avatar,
        id: user_id.clone(),
        username: username,
    };
    current_state.users.insert(user_id.clone(), user);
    let mut nick: Option<String> = None;
    match voice_state["nick"].as_str() {
        Some(st) => {
            nick = Some(st.to_string());
        }
        None => {}
    }
    match voice_state["voice_state"]["nick"].as_str() {
        Some(st) => {
            nick = Some(st.to_string());
        }
        None => {}
    }
    let mut talking = false;
    if current_state.voice_states.contains_key(&user_id.clone()) {
        talking = current_state
            .voice_states
            .get(&user_id.clone())
            .unwrap()
            .talking;
    }
    let vs = data::VoiceStateData {
        mute: voice_state["voice_state"]["mute"].as_bool().unwrap(),
        deaf: voice_state["voice_state"]["deaf"].as_bool().unwrap(),
        self_mute: voice_state["voice_state"]["self_mute"].as_bool().unwrap(),
        self_deaf: voice_state["voice_state"]["self_deaf"].as_bool().unwrap(),
        suppress: voice_state["voice_state"]["suppress"].as_bool().unwrap(),
        nick: nick,
        talking: talking,
    };
    current_state.voice_states.insert(user_id, vs);
}

async fn update_state_from_voice_state_list(
    state: Arc<Mutex<data::ConnState>>,
    voice_state_list: &Value,
) {
    for voice_state in voice_state_list.as_array().unwrap() {
        update_state_from_voice_state(state.clone(), voice_state).await;
    }
}

pub async fn connector(
    sender: Arc<Mutex<futures::channel::mpsc::Sender<data::ConnState>>>,
    recvr: Arc<Mutex<futures::channel::mpsc::Receiver<String>>>,
) {
    let state = Arc::new(Mutex::new(data::ConnState::new()));
    let debug_stdout = true;
    tokio::spawn(async move {
        loop {
            if debug_stdout {
                println!("Awaiting connection");
            }
            let req = Request::builder()
                .uri("ws://127.0.0.1:6463/?v=1&client_id=207646673902501888")
                .method("GET")
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", generate_key())
                .header("Host", "127.0.0.1")
                .header("Origin", "https://streamkit.discord.com")
                .body(())
                .unwrap();

            let ws_stream = match connect_async(req).await {
                Ok(a) => a.0,
                Err(_err) => {
                    // Retry in 1 second. Probably make it less often?
                    sleep(Duration::from_millis(1000)).await;
                    continue;
                }
            };
            if debug_stdout {
                println!("Connected to local Discord");
            }
            let (write, read) = ws_stream.split();
            let writer = Arc::new(Mutex::new(write));

            // Message thread to writer
            {
                let recvr = recvr.clone();
                let writer = writer.clone();
                tokio::spawn(async move {
                    while let Some(event) = recvr.clone().lock().await.next().await {
                        //println!("{}", event);
                        send_socket!(writer, [event]);
                    }
                });
            }

            read.for_each(|message| async {
                if message.is_err() {
                    println!("Connection to Discord lost");
                    state.lock().await.clear();
                    return;
                }
                let copy_state = state.lock().await.clone();
                let before_hash = data::calculate_hash(&copy_state);
                let message = message.unwrap();
                let writer = writer.clone();
                match message {
                    tungstenite::Message::Text(raw_data) => {
                        let data: Value = serde_json::from_str(&raw_data).unwrap();
                        // Data is a raw JSON object
                        //println!("{}", raw_data);
                        match data["cmd"].as_str().unwrap() {
                            "AUTHORIZE" => {
                                // Make HTTPS request to auth user
                                let url = "https://streamkit.discord.com/overlay/token";
                                let obj = json!({"code": data["data"]["code"]});
                                let resp: serde_json::Value = reqwest::Client::new()
                                    .post(url)
                                    .json(&obj)
                                    .send()
                                    .await
                                    .unwrap()
                                    .json()
                                    .await
                                    .unwrap();
                                match resp.get("access_token") {
                                    Some(value) => {
                                        send_socket!(writer, packet_auth2!(value));
                                    }
                                    None => {
                                        if debug_stdout {
                                            println!("No access token, failed to connect")
                                        }
                                        // TODO Reattempt connect
                                    }
                                }
                            }
                            "AUTHENTICATE" => match data["data"].get("access_token") {
                                None => {
                                    if debug_stdout {
                                        println!("Not authorized");
                                        println!("{:?}", data);
                                    }
                                }
                                Some(_value) => {
                                    send_socket!(writer, packet_req_all_guilds!());
                                    send_socket!(writer, packet_req_selected_voice!());
                                    send_socket!(writer, packet_sub_server!());
                                    match data["data"]["user"]["id"].as_str() {
                                        Some(value) => {
                                            state.lock().await.user_id = Some(value.to_string());
                                        }
                                        None => {
                                            state.lock().await.user_id = None;
                                        }
                                    }
                                }
                            },
                            "GET_GUILDS" => {}
                            "GET_SELECTED_VOICE_CHANNEL" => match data["data"].get("id") {
                                Some(value) => {
                                    state.lock().await.voice_channel =
                                        Some(value.as_str().unwrap().parse().unwrap());
                                    update_state_from_voice_state_list(
                                        state.clone(),
                                        &data["data"]["voice_states"],
                                    )
                                    .await;
                                    send_socket!(
                                        writer,
                                        packet_sub_voice_channel!(value.as_str().unwrap())
                                    );
                                }
                                None => {
                                    user_left_channel(state.clone()).await;
                                }
                            },
                            "DISPATCH" => {
                                match data["evt"].as_str().unwrap() {
                                    "READY" => {
                                        send_socket!(writer, packet_auth!("207646673902501888"));
                                    }
                                    "SPEAKING_START" => {
                                        let id = data["data"]["user_id"]
                                            .as_str()
                                            .unwrap()
                                            .to_string()
                                            .clone();
                                        if state.lock().await.voice_channel.is_none() {
                                            send_socket!(writer, packet_req_selected_voice!());
                                        }
                                        set_user_talking(state.clone(), id, true).await;
                                    }
                                    "SPEAKING_STOP" => {
                                        let id = data["data"]["user_id"]
                                            .as_str()
                                            .unwrap()
                                            .to_string()
                                            .clone();
                                        set_user_talking(state.clone(), id, false).await;
                                    }
                                    "VOICE_STATE_DELETE" => {
                                        //println!("{:?}", data);
                                        let id = data["data"]["user"]["id"].clone();
                                        let user_id =
                                            state.lock().await.user_id.as_ref().unwrap().clone();
                                        if id == user_id {
                                            user_left_channel(state.clone()).await;
                                        }
                                    }
                                    "VOICE_STATE_CREATE" => {
                                        if state.lock().await.voice_channel.is_none() {
                                            send_socket!(writer, packet_req_selected_voice!());
                                        }
                                    }
                                    "VOICE_STATE_UPDATE" => {
                                        let _id = data["data"]["user"]["id"].clone();
                                        update_state_from_voice_state(state.clone(), &data["data"])
                                            .await;
                                    }
                                    "VOICE_CHANNEL_SELECT" => {
                                        // User has manually chosen to join a room
                                        send_socket!(writer, packet_req_selected_voice!());
                                        // Let's ask for more info
                                    }
                                    "VOICE_CONNECTION_STATUS" => {
                                        if debug_stdout {
                                            // TODO Potentially make this part of the conn state
                                            // But be aware that allowing this to change the state will
                                            // Cause the overlay to render every couple of seconds for no
                                            // effect
                                            println!("{}: {}", data["evt"], data["data"]["state"]);
                                        }
                                    }
                                    _ => {
                                        if debug_stdout {
                                            println!("{:?}", data);
                                        }
                                    }
                                }
                            }
                            _ => {
                                if debug_stdout {
                                    println!("{:?}", data);
                                }
                            }
                        }
                        let copy_state = state.lock().await.clone();
                        if before_hash != data::calculate_hash(&copy_state) {
                            match sender.lock().await.try_send(copy_state) {
                                Ok(_) => {}
                                Err(_e) => {}
                            }
                        }
                    }
                    tungstenite::Message::Binary(_raw_data) => {}
                    tungstenite::Message::Ping(_raw_data) => {}
                    tungstenite::Message::Pong(_raw_data) => {}
                    tungstenite::Message::Frame(_raw_data) => {}
                    tungstenite::Message::Close(_raw_data) => {
                        state.lock().await.clear();
                    }
                }
            })
            .await;
        }
    });
}
