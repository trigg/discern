extern crate clap;
extern crate serde_json;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use std::sync::Arc;

mod core;
mod data;
mod macros;

use crate::data::ConnState;

#[tokio::main]
async fn main() {
    // Websocket events to main thread
    let (event_sender, event_recv) = futures::channel::mpsc::channel::<ConnState>(0);
    let event_sender = Arc::new(Mutex::new(event_sender));
    let event_recv = Arc::new(Mutex::new(event_recv));

    // Main thread messages to Websocket output
    let (msg_sender, msg_recv) = futures::channel::mpsc::channel::<String>(0);
    let _msg_sender = Arc::new(Mutex::new(msg_sender));
    let msg_recv = Arc::new(Mutex::new(msg_recv));

    // Start a thread for connection
    let connector_event_sender = event_sender.clone();
    let connector_msg_recv = msg_recv.clone();
    core::connector(connector_event_sender.clone(), connector_msg_recv.clone()).await;

    // Start our own loop - just print it
    let mut state = ConnState::new();
    loop {
        while let Some(event) = event_recv.lock().await.next().await {
            state.replace_self(event);
            println!("{:?}", state);
        }
    }
}
