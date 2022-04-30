use futures::lock::Mutex;
use futures_util::{SinkExt, StreamExt};
use http::Request;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::handshake::client::generate_key;
extern crate serde_json;
use serde_json::json;
use serde_json::Value;
use std::collections::hash_map::HashMap;
use std::sync::Arc;
extern crate clap;
use clap::{arg, command, Command};
mod clispam;
mod data;
#[macro_use]
mod macros;
mod rpc;
mod wlroots;
mod x11;

#[tokio::main]
async fn main() {
    let matches = command!()
                    .subcommand_required(true)
                    .subcommand(
                        Command::new("auto")
                            .about("Autodetect mode based on ENV variables"))
                    .subcommand(
                        Command::new("x11")
                            .about("Create an overlay window with x11"))
                    .subcommand(
                        Command::new("wlroots")
                            .about("Create an overlay window with wlroots layer_shell"))
                    .subcommand(
                        Command::new("clispam")
                            .about("Output to stdout. A lot"))
                    .subcommand(
                        Command::new("rpc")
                            .about("Poll data from Discord, await and answer, and reply on stdout")
                            .subcommand(
                                Command::new("channel")
                                .about("Get current channel information")
                                .subcommand(
                                    Command::new("id")
                                    .about("Get Room ID. None is 0")
                                )
                                .subcommand(
                                    Command::new("name")
                                    .about("Get Room Name")
                                )
                                .subcommand(
                                    Command::new("useridlist")
                                    .about("Get List of users in room, return IDs")
                                )
                                .subcommand(
                                    Command::new("usernamelist")
                                    .about("Get List of users in room, return names")
                                )
                                .subcommand(
                                    Command::new("move")
                                    .about("Switch to another room by ID")
                                    .arg(arg!([ID] "ID of room to move user to"))
                                )
                            )
                            .subcommand(
                                Command::new("devices")
                                .about("Get audio device information")
                                .subcommand(
                                    Command::new("mute")
                                    .about("Check mute state of user")
                                    .arg(arg!(-s --set <VALUE> "Alter mute state. `true` `false` or `toggle`").required(false))
                                )
                                .subcommand(
                                    Command::new("deaf")
                                    .about("Check deaf state of user")
                                    .arg(arg!(-s --set <VALUE> "Alter deaf state. `true` `false` or `toggle`").required(false))                                    
                                )
                            )
                        )
                    .subcommand(
                        Command::new("statefile")
                            .about("Write state into a file every time it changes.")
                            .arg(arg!(-f --file <FILE> "The location of the file to write to. Defaults to ~/.discern-state").allow_invalid_utf8(true))
                    )
                    .subcommand(
                        Command::new("gamescope")
                            .about("Create an overlay in gamescope. Reads changes in ~/.discern.control")
                    )
                    .get_matches();

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

    let (ws_stream, _) = connect_async(req).await.expect("Failed to connect");

    let (write, read) = ws_stream.split();

    let state = data::ConnState {
        user_id: None,
        voice_channel: None,
        users: HashMap::new(),
        voice_states: HashMap::new(),
    };

    let writer = Arc::new(Mutex::new(write));
    let state = Arc::new(Mutex::new(state));
    let mut exit_on_disconnect = true;
    let mut debug_stdout = false;

    let mut callback: Arc<dyn Fn(String)> = Arc::new(|_string| {});

    match matches.subcommand() {
        Some(("auto", _sub_matches)) => {
            println!("Auto: Not yet implemented");
            std::process::exit(0);
        }
        Some(("x11", _sub_matches)) => {
            exit_on_disconnect = false;
            callback = Arc::new(x11::start(state.clone()));
        }
        Some(("wlroots", _sub_matches)) => {
            exit_on_disconnect = false;
            callback = Arc::new(wlroots::start(state.clone()));
        }
        Some(("clispam", _sub_matches)) => {
            exit_on_disconnect = true;
            debug_stdout = true;
            callback = Arc::new(clispam::start(state.clone()));
        }
        Some(("rpc", _sub_matches)) => {
            exit_on_disconnect = true;
            debug_stdout = false;
            callback = Arc::new(rpc::start(writer.clone(), _sub_matches, state.clone()));
        }
        Some(("statefile", _sub_matches)) => {
            //exit_on_disconnect = false;

            println!("StateFile: Not yet implemented");
            std::process::exit(0);
        }
        Some((_, _)) => {
            println!("Other: Not yet implemented");
            std::process::exit(0);
        }
        None => {}
    }
    let websocket_loop = {
        read.for_each(|message| async {
            let message = message.unwrap();
            let writer = writer.clone();
            match message {
                tungstenite::Message::Text(raw_data) => {
                    let data: Value = serde_json::from_str(&raw_data).unwrap();
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
                                    send_auth2!(writer, value);
                                }
                                None => {
                                    if debug_stdout {
                                        println!("No access token, failed to connect")
                                    }
                                    if exit_on_disconnect {
                                        std::process::exit(0);
                                    }
                                    // TODO Reattempt connect
                                }
                            }
                        }
                        "AUTHENTICATE" => {
                            match data["data"].get("access_token") {
                                None => {
                                    if debug_stdout {
                                        println!("Not authorized");
                                        println!("{:?}", data);
                                    }
                                    if exit_on_disconnect {
                                        std::process::exit(0);
                                    }
                                    // TODO Reattempt connection
                                }
                                Some(_value) => {
                                    send_req_all_guilds!(writer);
                                    send_req_selected_voice!(writer);
                                    send_sub_server!(writer);
                                    match data["data"]["user"]["id"].as_str() {
                                        Some(value) => {
                                            state.lock().await.user_id = Some(value.to_string());
                                        }
                                        None => {
                                            state.lock().await.user_id = None;
                                        }
                                    }
                                }
                            }
                        }
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
                                send_sub_voice_channel!(writer, value.as_str().unwrap());
                            }
                            None => {
                                user_left_channel(state.clone()).await;
                            }
                        },
                        "DISPATCH" => {
                            match data["evt"].as_str().unwrap() {
                                "READY" => {
                                    send_auth!(writer, "207646673902501888");
                                }
                                "SPEAKING_START" => {
                                    let id = data["data"]["user_id"]
                                        .as_str()
                                        .unwrap()
                                        .to_string()
                                        .clone();
                                    if state.lock().await.voice_channel.is_none() {
                                        send_req_selected_voice!(writer);
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
                                    println!("{:?}", data);
                                    let id = data["data"]["user"]["id"].clone();
                                    let user_id =
                                        state.lock().await.user_id.as_ref().unwrap().clone();
                                    if id == user_id {
                                        user_left_channel(state.clone()).await;
                                    }
                                }
                                "VOICE_STATE_CREATE" => {
                                    let _id = data["data"]["user"]["id"].clone();
                                    if state.lock().await.voice_channel.is_none() {
                                        send_req_selected_voice!(writer);
                                    }
                                }
                                "VOICE_STATE_UPDATE" => {
                                    let _id = data["data"]["user"]["id"].clone();
                                    update_state_from_voice_state(state.clone(), &data["data"])
                                        .await;
                                }
                                "VOICE_CHANNEL_SELECT" => {
                                    // User has manually chosen to join a room
                                    send_req_selected_voice!(writer);
                                    // Let's ask for more info
                                }
                                "VOICE_CONNECTION_STATUS" => {
                                    if debug_stdout {
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
                    (callback)(raw_data);
                }
                tungstenite::Message::Binary(_raw_data) => {}
                tungstenite::Message::Ping(_raw_data) => {}
                tungstenite::Message::Pong(_raw_data) => {}
                tungstenite::Message::Frame(_raw_data) => {}
                tungstenite::Message::Close(_raw_data) => {
                    if exit_on_disconnect {
                        std::process::exit(0);
                    }
                }
            }
        })
    };
    websocket_loop.await;
}

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
    let avatar = voice_state["user"]["avatar"].as_str().unwrap().to_string();

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
