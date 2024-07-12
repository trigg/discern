extern crate clap;
extern crate serde_json;
use clap::{arg, command, Command};
use futures::lock::Mutex;
use futures::stream::StreamExt;
use serde_json::json;
use std::sync::Arc;

mod core;
mod data;
mod macros;

#[tokio::main]
async fn main() {
    // Websocket events to main thread
    let (event_sender, event_recv) = futures::channel::mpsc::channel::<data::ConnState>(10);
    let event_sender = Arc::new(Mutex::new(event_sender));
    let event_recv = Arc::new(Mutex::new(event_recv));

    // Main thread messages to Websocket output
    let (msg_sender, msg_recv) = futures::channel::mpsc::channel::<String>(10);
    let msg_sender = Arc::new(Mutex::new(msg_sender));
    let msg_recv = Arc::new(Mutex::new(msg_recv));

    // Start a thread for connection
    let connector_event_sender = event_sender.clone();
    let connector_msg_recv = msg_recv.clone();
    core::connector(connector_event_sender.clone(), connector_msg_recv.clone()).await;

    // Setup Command line args
    let matches = command!()
        .subcommand_required(true)
        .subcommand(
            Command::new("channel")
                .about("Get current channel information")
                .subcommand(Command::new("id").about("Get Room ID. None is 0"))
                .subcommand(Command::new("name").about("Get Room Name"))
                .subcommand(
                    Command::new("useridlist").about("Get List of users in room, return IDs"),
                )
                .subcommand(
                    Command::new("usernamelist").about("Get List of users in room, return names"),
                )
                .subcommand(
                    Command::new("move")
                        .about("Switch to another room by ID")
                        .arg(arg!([ID] "ID of room to move user to")),
                ),
        )
        .subcommand(
            Command::new("devices")
                .about("Get audio device information")
                .subcommand(
                    Command::new("mute").about("Check mute state of user").arg(
                        arg!(-s --set <VALUE> "Alter mute state. `true` `false` or `toggle`")
                            .required(false),
                    ),
                )
                .subcommand(
                    Command::new("deaf").about("Check deaf state of user").arg(
                        arg!(-s --set <VALUE> "Alter deaf state. `true` `false` or `toggle`")
                            .required(false),
                    ),
                ),
        )
        .get_matches();
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

    loop {
        while let Some(event) = event_recv.lock().await.next().await {
            println!("{:?}", event);
            //let data: Value = serde_json::from_str(&event).unwrap();
            let data = json!([]); // TODO
                                  // We broke RPC as there is no direct JSON at this point.
                                  // Need to consider raw-string options again
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
                            send_mpsc!(
                                msg_sender,
                                packet_set_devices!(
                                    "mute",
                                    !data["data"]["mute"].as_bool().unwrap(),
                                    "deadbeef"
                                )
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
                            send_mpsc!(
                                msg_sender,
                                packet_set_devices!(
                                    "deaf",
                                    !data["data"]["deaf"].as_bool().unwrap(),
                                    "deadbeef"
                                )
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
                            send_mpsc!(msg_sender, packet_set_devices!("mute", true, "deadbeef"));
                        }
                        Some(AudioAction::False) => {
                            send_mpsc!(msg_sender, packet_set_devices!("mute", false, "deadbeef"));
                        }
                        Some(AudioAction::Toggle) => {
                            send_mpsc!(msg_sender, packet_req_devices!());
                        }
                        Some(AudioAction::Get) => {
                            send_mpsc!(msg_sender, packet_req_devices!());
                        }
                        None => {}
                    }
                    match user_args.deaf {
                        Some(AudioAction::True) => {
                            send_mpsc!(msg_sender, packet_set_devices!("deaf", true, "deadbeef"));
                        }
                        Some(AudioAction::False) => {
                            send_mpsc!(msg_sender, packet_set_devices!("deaf", false, "deadbeef"));
                        }
                        Some(AudioAction::Toggle) => {
                            send_mpsc!(msg_sender, packet_req_devices!());
                        }
                        Some(AudioAction::Get) => {
                            send_mpsc!(msg_sender, packet_req_devices!());
                        }
                        None => {}
                    }
                    match user_args.set_room {
                        Some(ref roomid) => {
                            send_mpsc!(msg_sender, packet_set_channel!(roomid.clone()));
                        }
                        None => {}
                    }
                }
                Some(_) => {}
                None => {}
            }
        }
    }
}
