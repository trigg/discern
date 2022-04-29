use crate::data::ConnState;
use futures::lock::Mutex;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub fn start(gui_state: Arc<Mutex<ConnState>>) -> impl Fn(String) {
    let _gui_loop = tokio::task::spawn(async move {
        loop {
            // Fake GUI. Output state to TERM
            let state = gui_state.lock().await.clone();
            println!("TICK");
            println!("User ID : {:?}", state.user_id);
            println!("Voice Channel : {:?}", state.voice_channel);
            for key in state.voice_states.keys() {
                let vs = state.voice_states.get(key).unwrap().clone();
                let user = state.users.get(key).unwrap().clone();

                println!(
                    "{} : Mute({}) Deaf({:}) Talking({})",
                    match vs.nick {
                        Some(val) => val,
                        None => user.username,
                    },
                    vs.mute || vs.self_mute,
                    vs.deaf || vs.self_deaf,
                    vs.talking
                );
            }
            sleep(Duration::from_millis(1000)).await;
        }
    });

    |value| {
        println!("{}", value);
    }
}
