use futures_util::{StreamExt, SinkExt};
use tokio::time::{Duration, sleep};
use tokio::io::{AsyncReadExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::handshake::client::generate_key;
use http::{Request};
extern crate serde_json;
use serde_json::Value;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::vec::Vec;
use std::collections::hash_map::HashMap;
extern crate clap;
use clap::{arg,command,Command};
mod data;

macro_rules! send{
    ($writer: expr, $value: expr) => {
        $writer.lock().unwrap().send(Message::Text($value.to_string()+"\n")).await.unwrap();
    }
}

macro_rules! send_auth{
    ($writer: expr, $auth_code: expr) => {
        send!($writer, json!({
            "cmd": "AUTHORIZE",
            "args": {
                "client_id": $auth_code,
                "scopes": ["rpc", "messages.read"],
                "prompt": "none",
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_auth2{
    ($writer: expr, $token: expr) => {
        send!($writer, json!({
            "cmd": "AUTHENTICATE",
            "args": {
                "access_token": $token,
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_req_all_guilds{
    {$writer: expr} => {
        send!($writer, json!({
            "cmd": "GET_GUILDS",
            "args": {
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_req_selected_voice{
    {$writer: expr} => {
        send!($writer, json!({
            "cmd": "GET_SELECTED_VOICE_CHANNEL",
            "args": {
            },
            "nonce": "deadbeef"
        }));
    }
}

macro_rules! send_sub{
    {$writer: expr, $event: expr, $args: expr, $nonce: expr} =>{
        send!($writer, json!({
            "cmd": "SUBSCRIBE",
            "args": $args,
            "evt": $event,
            "nonce": $nonce
        }));
    }
}

macro_rules! send_sub_server{
    {$writer: expr} => {
        send_sub!($writer.clone(), "VOICE_CHANNEL_SELECT", json!({}), "VOICE_CHANNEL_SELECT");
        send_sub!($writer.clone(), "VOICE_CONNECTION_STATUS", json!({}), "VOICE_CONNECTION_STATUS");
    }
}

macro_rules! send_sub_channel{
    {$writer: expr, $event: expr, $channel: expr} => {
        send_sub!($writer, $event, json!({"channel_id":$channel}), $channel);
    }
}

macro_rules! send_sub_voice_channel{
    {$writer: expr, $channel: expr} => {
        send_sub_channel!($writer.clone(), "VOICE_STATE_CREATE", $channel);
        send_sub_channel!($writer.clone(), "VOICE_STATE_UPDATE", $channel);
        send_sub_channel!($writer.clone(), "VOICE_STATE_DELETE", $channel);
        send_sub_channel!($writer.clone(), "SPEAKING_START", $channel);
        send_sub_channel!($writer.clone(), "SPEAKING_STOP", $channel);
    }
}


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
                                    .about("Switch to another roomm by ID")
                                    .arg(arg!([ID] "ID of room to move user to"))
                                )
                            )
                            .subcommand(
                                Command::new("devices")
                                .about("Get audio device information")
                                .subcommand(
                                    Command::new("mute")
                                    .about("Check mute state of user")
                                    .arg(arg!(-s --set <VALUE> "Alter mute state. `true` `false` or `toggle`"))
                                )
                                .subcommand(
                                    Command::new("deaf")
                                    .about("Check deaf state of user")
                                    .arg(arg!(-s --set <VALUE> "Alter deaf state. `true` `false` or `toggle`"))
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

    match matches.subcommand() {
        Some(("auto", _sub_matches)) => {
            println!("AUTO");
        }
        Some(("x11", _sub_matches)) => {
            println!("X11");
        }
        Some(("wlroots", _sub_matches)) => {
            println!("wlroots");
        }
        Some(("clispam", _sub_matches)) => {
            println!("clispam");
        }
        Some(("rpc", _sub_matches)) => {
            println!("rpc");
        }
        Some(("statefile", _sub_matches)) => {
            println!("statefile");
        }
        Some((&_, _)) => {
            println!("What is happening?");
        }
        None => {
            println!("uhoh");
        }
    }
                    
    let req = Request::builder()
       .uri("ws://127.0.0.1:6463/?v=1&client_id=207646673902501888")
       .method("GET")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key())
        .header("Host","127.0.0.1")
        .header("Origin", "https://streamkit.discord.com")

       .body(())
       .unwrap();

    let (stdin_tx, _stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    let (ws_stream, _) = connect_async(req).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    let state = data::ConnState{
        user_id : None,
        voice_channel: None,
        users : HashMap::new(),
        voice_states: HashMap::new(),
        errors: Vec::new(),
    };

    let writer = Arc::new(Mutex::new(write));
    let state = Arc::new(Mutex::new(state));
    let gui_state = state.clone();

    // GUI loop
    let _gui_loop = tokio::task::spawn(async move {
        loop {
            // Fake GUI. Output state to TERM
            let state = gui_state.lock().unwrap().clone();
            println!("TICK");
            println!("User ID : {:?}",state.user_id);
            println!("Voice Channel : {:?}", state.voice_channel);
            for key in state.voice_states.keys() {
                let vs = state.voice_states.get(key).unwrap().clone();
                let user = state.users.get(key).unwrap().clone();

                println!("{} : Mute({}) Deaf({:}) Talking({})", 
                    match vs.nick { Some(val) => val, None => user.username},
                    vs.mute || vs.self_mute,
                    vs.deaf || vs.self_deaf,
                    vs.talking);
            }
            sleep(Duration::from_millis(1000)).await;
        }
    });


    let ws_to_stdout = {
        read.for_each(|message| async {
            let message = message.unwrap();
            let writer = writer.clone();
            match message{
                tungstenite::Message::Text(raw_data) => {
                    let data:Value = serde_json::from_str(&raw_data).unwrap();

                    match data["cmd"].as_str().unwrap(){
                        "AUTHORIZE" => {
                            println!( "AUTH Stage 1");
                            println!( "{:?}",data["data"]["code"]);

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
                              .await.unwrap();
                            println!("{:?}", resp);
                            match resp.get("access_token") {
                                Some(value) => {
                                    send_auth2!(writer, value);
                                }
                                None => {
                                    println!("Access token missing in response");
                                    println!("{:?}",resp);
                                }
                            }
                        }
                        "AUTHENTICATE" => {
                            println!("AUTH Stage 2");
                            match data["data"].get("access_token") {
                                None => {
                                    println!("Not authorized");
                                    println!("{:?}",data);
                                }
                                Some(_value) => {
                                    send_req_all_guilds!(writer);
                                    send_req_selected_voice!(writer);
                                    send_sub_server!(writer);
                                    println!("{:?}",data);
                                    match data["data"]["user"]["id"].as_str(){
                                        Some(value) => {
                                            println!("User ID");
                                            state.lock().unwrap().user_id = Some(value.to_string());
                                        }
                                        None => {
                                            println!("No user logged in");
                                            state.lock().unwrap().user_id = None;
                                        }
                                    }

                                }
                            }
                        }
                        "GET_GUILDS" => {

                        }
                        "GET_SELECTED_VOICE_CHANNEL" => {

                            match data["data"].get("id") {
                                Some(value) => {
                                    state.lock().unwrap().voice_channel = Some(value.as_str().unwrap().parse().unwrap());
                                    println!("Found user in channel");
                                    update_state_from_voice_state_list(state.clone(), &data["data"]["voice_states"]).await;
                                    send_sub_voice_channel!(writer, value.as_str().unwrap());
                                }
                                None => {
                                    user_left_channel(state.clone()).await;
                                }
                            }
                        }
                        "DISPATCH" => {
                            match data["evt"].as_str().unwrap() {
                                "READY" => {
                                    println!("Connection started");
                                    send_auth!(writer, "207646673902501888");
                                }
                                "SPEAKING_START" => {
                                    let id = data["data"]["user_id"].as_str().unwrap().to_string().clone();
                                    if state.lock().unwrap().voice_channel.is_none() {
                                        send_req_selected_voice!(writer);
                                    }
                                    set_user_talking(state.clone(), id, true).await;
                                    println!("{:?}",data);

                                }
                                "SPEAKING_STOP" => {
                                    let id = data["data"]["user_id"].as_str().unwrap().to_string().clone();
                                    set_user_talking(state.clone(), id, false).await;
                                    println!("{:?}",data);

                                }
                                "VOICE_STATE_DELETE" => {
                                    println!("{:?}",data);
                                    let id = data["data"]["user"]["id"].clone();
                                    let user_id = state.lock().unwrap().user_id.as_ref().unwrap().clone();
                                    if id == user_id{
                                        user_left_channel(state.clone()).await;
                                    }
                                }
                                "VOICE_STATE_CREATE" => {
                                    let _id = data["data"]["user"]["id"].clone();
                                    if state.lock().unwrap().voice_channel.is_none() {
                                        send_req_selected_voice!(writer);
                                    }
                                    println!("{:?}",data);
                                }
                                "VOICE_STATE_UPDATE" => {
                                    let _id = data["data"]["user"]["id"].clone();
                                    println!("{:?}",data);
                                    update_state_from_voice_state(state.clone(), &data["data"]).await;
                                }
                                "VOICE_CHANNEL_SELECT" => {
                                    // User has manually chosen to join a room
                                    send_req_selected_voice!(writer);
                                    // Let's ask for more info
                                }
                                "VOICE_CONNECTION_STATUS" => {
                                    println!("{}: {}", data["evt"], data["data"]["state"]);
                                }
                                _ => {
                                    println!("{:?}",data);
                                }
                            }

                        }
                        _ => {
                            println!("{:?}",data);
                        }
                    }
                }
                tungstenite::Message::Binary(_raw_data) => {
                    println!("Binary recv");
                }
                tungstenite::Message::Ping(_raw_data) => {
                    println!("Ping recv");
                }
                tungstenite::Message::Pong(_raw_data) => {
                    println!("Pong recv");
                }
                tungstenite::Message::Frame(_raw_data)=>{
                    println!("Frame recv");
                }
                tungstenite::Message::Close(_raw_data)=>{
                    println!("Close recv");
                }
            }
        })
    };
    println!("Starting\n");
    ws_to_stdout.await;
}

async fn user_left_channel(state:  Arc<Mutex<data::ConnState>>){
    let mut current_state = state.lock().unwrap();
    current_state.voice_channel = None;
    current_state.users.clear();
    current_state.voice_states.clear();
}

async fn set_user_talking(state:  Arc<Mutex<data::ConnState>>, user_id: String, talking: bool){
    let mut unlocked = state.lock().unwrap();
    let mut voice_state = unlocked.voice_states.get_mut(&user_id).unwrap().clone();
    voice_state.talking = talking;
    println!("{}", talking);
    unlocked.voice_states.insert(user_id.clone(),voice_state);
}

async fn update_state_from_voice_state (state : Arc<Mutex<data::ConnState>>, voice_state: &Value){
    let user_id : String  = voice_state["user"]["id"].as_str().unwrap().to_string();
    let mut current_state = state.lock().unwrap();

    let username = voice_state["user"]["username"].as_str().unwrap().to_string();
    let avatar = voice_state["user"]["avatar"].as_str().unwrap().to_string();
    println!("{} {}", user_id, username);

    println!( "Inserting user");
    let user = data::DiscordUserData{
        avatar: avatar,
        id: user_id.clone(),
        username: username
    };
    current_state.users.insert(user_id.clone(), user);
    
    let mut nick: Option<String> = None;
    match voice_state["nick"].as_str(){
        Some(st) => {
            nick = Some(st.to_string());
        }
        None => {
        }
    }
    match voice_state["voice_state"]["nick"].as_str(){
        Some(st) => {
            nick = Some(st.to_string());
        }
        None => {
        }
    }
    let mut talking = false;
    if current_state.voice_states.contains_key(&user_id.clone()) {
        talking = current_state.voice_states.get(&user_id.clone()).unwrap().talking;
    }
    let vs = data::VoiceStateData{
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

async fn update_state_from_voice_state_list (state: Arc<Mutex<data::ConnState>>, voice_state_list: &Value) {
    for voice_state in voice_state_list.as_array().unwrap() {
        update_state_from_voice_state(state.clone(), voice_state).await;
    }
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}
