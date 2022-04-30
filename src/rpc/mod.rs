use crate::data::ConnState;
use futures::lock::Mutex;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
extern crate serde_json;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use serde_json::json;
use serde_json::Value;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

pub fn start(
    writer: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>>>,
    matches: &clap::ArgMatches,
    _gui_state: Arc<Mutex<ConnState>>,
) -> impl Fn(String) {
    let writer = writer.clone();
    let _gui_loop = tokio::task::spawn(async move {
        loop {
            // We're doing nothing
            sleep(Duration::from_millis(10000)).await;
            println!("Timed out error");
            std::process::exit(1);
        }
    });

    // Types to store what the user requested action was
    #[derive(Debug, Clone)]
    enum AudioAction {
        True,
        False,
        Toggle,
        Get,
    }
    #[derive(Debug, Clone)]
    struct Args {
        get_room_id: bool,
        get_room_name: bool,
        get_room_userlist: bool,
        get_room_idlist: bool,
        set_room: Option<String>,
        mute: Option<AudioAction>,
        deaf: Option<AudioAction>,
    }
    let mut user_args = Args {
        get_room_id: false,
        get_room_name: false,
        get_room_userlist: false,
        get_room_idlist: false,
        set_room: None,
        mute: None,
        deaf: None,
    };
    // Decant the args into the store above
    match matches.subcommand() {
        Some(("channel", sub)) => match sub.subcommand() {
            Some(("id", _)) => user_args.get_room_id = true,
            Some(("name", _)) => user_args.get_room_name = true,
            Some(("useridlist", _)) => user_args.get_room_idlist = true,
            Some(("usernamelist", _)) => user_args.get_room_userlist = true,
            Some(("move", sub)) => {
                user_args.set_room = Some(sub.value_of("ID").unwrap().to_string());
            }
            Some((_, _)) => {
                println!("Unknown rpc args");
                std::process::exit(0);
            }
            None => {
                println!("Unknown rpc args");
                std::process::exit(0);
            }
        },
        Some(("devices", sub)) => match sub.subcommand() {
            Some(("mute", args)) => match args.value_of("set") {
                Some("true") => user_args.mute = Some(AudioAction::True),
                Some("false") => user_args.mute = Some(AudioAction::False),
                Some("toggle") => user_args.mute = Some(AudioAction::Toggle),
                Some(_) => {
                    println!("Unknown rpc args");
                    std::process::exit(0);
                }
                None => user_args.mute = Some(AudioAction::Get),
            },
            Some(("deaf", args)) => match args.value_of("set") {
                Some("true") => user_args.deaf = Some(AudioAction::True),
                Some("false") => user_args.deaf = Some(AudioAction::False),
                Some("toggle") => user_args.deaf = Some(AudioAction::Toggle),
                Some(_) => {
                    println!("Unknown rpc args");
                    std::process::exit(0);
                }
                None => user_args.deaf = Some(AudioAction::Get),
            },
            Some((_, _)) => {
                println!("Unknown rpc args");
                std::process::exit(0);
            }
            None => {
                println!("Unknown rpc args");
                std::process::exit(0);
            }
        },
        Some((_, _)) => {
            println!("Unknown rpc args");
            std::process::exit(0);
        }
        None => {
            println!("Unknown rpc args");
            std::process::exit(0);
        }
    }
    let writer = writer.clone();

    // Return a function to be called every time we receive a new packet
    // Due to the simplicity of passing a String over a JSON Value it needs to
    // be decoded a 2nd time. Not great.
    move |value| {
        // clone all data and spawn an async task
        let value = value.clone();
        let writer = writer.clone();
        let user_args = user_args.clone();
        tokio::task::spawn(async move {
            let writer = writer.clone();
            let data: Value = serde_json::from_str(&value).unwrap();
            match data["cmd"].as_str() {
                Some("SELECT_VOICE_CHANNEL") => {
                    // Successfully Selected a channel. Exit!
                    std::process::exit(0);
                }
                Some("GET_SELECTED_VOICE_CHANNEL") => {
                    if user_args.get_room_id {
                        // Print out the channel ID and exit!
                        match data["data"]["id"].as_str() {
                            Some(val) => {
                                println!("{}", val);
                            }
                            None => {
                                println!("0");
                            }
                        }
                        std::process::exit(0);
                    }
                    if user_args.get_room_name {
                        // Print out the channel name and exit!
                        match data["data"]["name"].as_str() {
                            Some(val) => {
                                println!("{}", val);
                            }
                            None => {
                                println!("");
                            }
                        }
                        std::process::exit(0);
                    }
                    if user_args.get_room_idlist {
                        // Print out the user id list and exit!
                        match data["data"]["voice_states"].as_array() {
                            Some(array) => {
                                for user in array {
                                    println!("{}", user["user"]["id"].as_str().unwrap());
                                }
                            }
                            None => {}
                        }
                        std::process::exit(0);
                    }
                    if user_args.get_room_userlist {
                        // Print out the user name list and exit!
                        match data["data"]["voice_states"].as_array() {
                            Some(array) => {
                                for user in array {
                                    println!("{}", user["user"]["username"].as_str().unwrap());
                                }
                            }
                            None => {}
                        }
                        std::process::exit(0);
                    }
                }
                Some("SET_VOICE_SETTINGS") => {
                    // Successfully set voice settings. Exit!
                    std::process::exit(0);
                }
                Some("GET_VOICE_SETTINGS") => {
                    // Got current voice settings.
                    // if we're polling this, return and exit!
                    // if we're toggling, pass it back inverted
                    match user_args.mute.clone() {
                        Some(AudioAction::True) => {}
                        Some(AudioAction::False) => {}
                        Some(AudioAction::Toggle) => {
                            send_set_devices!(
                                writer,
                                "mute",
                                !data["data"]["mute"].as_bool().unwrap(),
                                "deadbeef"
                            );
                        }
                        Some(AudioAction::Get) => {
                            println!("{}", data["data"]["mute"].as_bool().unwrap());
                            std::process::exit(0);
                        }
                        None => {}
                    };
                    match user_args.deaf.clone() {
                        Some(AudioAction::True) => {}
                        Some(AudioAction::False) => {}
                        Some(AudioAction::Toggle) => {
                            send_set_devices!(
                                writer,
                                "deaf",
                                !data["data"]["deaf"].as_bool().unwrap(),
                                "deadbeef"
                            );
                        }
                        Some(AudioAction::Get) => {
                            println!("{}", data["data"]["deaf"].as_bool().unwrap());
                            std::process::exit(0);
                        }
                        None => {}
                    }
                }
                Some("AUTHENTICATE") => {
                    // On connection make first call.
                    match user_args.mute {
                        Some(AudioAction::True) => {
                            send_set_devices!(writer, "mute", true, "deadbeef");
                        }
                        Some(AudioAction::False) => {
                            send_set_devices!(writer, "mute", false, "deadbeef");
                        }
                        Some(AudioAction::Toggle) => {
                            send_req_devices!(writer);
                        }
                        Some(AudioAction::Get) => {
                            send_req_devices!(writer);
                        }
                        None => {}
                    }
                    match user_args.deaf {
                        Some(AudioAction::True) => {
                            send_set_devices!(writer, "deaf", true, "deadbeef");
                        }
                        Some(AudioAction::False) => {
                            send_set_devices!(writer, "deaf", false, "deadbeef");
                        }
                        Some(AudioAction::Toggle) => {
                            send_req_devices!(writer);
                        }
                        Some(AudioAction::Get) => {
                            send_req_devices!(writer);
                        }
                        None => {}
                    }
                    match user_args.set_room {
                        Some(roomid) => {
                            send_set_channel!(writer, roomid);
                        }
                        None => {
                            
                        }
                    }
                }
                Some(_) => {}
                None => {}
            }
        });
    }
}
