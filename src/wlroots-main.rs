extern crate clap;
extern crate serde_json;
use crate::data::calculate_hash;
use crate::data::ConnState;
use bytes::Bytes;
use cairo::{
    Antialias, Context, FillRule, FontSlant, FontWeight, ImageSurface, Operator, RectangleInt,
    Region,
};
use futures::lock::Mutex;
use futures::stream::StreamExt;
use gio::prelude::*;
use glib;
use gtk::prelude::*;
use gtk_layer_shell;
use std::collections::hash_map::HashMap;
use std::f64::consts::PI;
use std::io::Cursor;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

mod core;
mod data;
mod macros;

#[tokio::main]
async fn main() {
    let state = data::ConnState::new();

    let state = Arc::new(Mutex::new(state));
    let gui_state = state.clone();

    // Websocket events to main thread
    let (event_sender, event_recv) = futures::channel::mpsc::channel::<String>(0);
    let event_sender = Arc::new(Mutex::new(event_sender));
    let event_recv = Arc::new(Mutex::new(event_recv));

    // Main thread messages to Websocket output
    let (msg_sender, msg_recv) = futures::channel::mpsc::channel::<String>(0);
    let _msg_sender = Arc::new(Mutex::new(msg_sender));
    let msg_recv = Arc::new(Mutex::new(msg_recv));

    // Start a thread for connection
    let connector_state = state.clone();
    let connector_event_sender = event_sender.clone();
    let connector_msg_recv = msg_recv.clone();
    core::connector(
        connector_state.clone(),
        connector_event_sender.clone(),
        connector_msg_recv.clone(),
    )
    .await;

    // Avatar grabbing thread
    let state = Arc::new(std::sync::Mutex::new(ConnState::new()));

    let avatar_list_raw: HashMap<String, Option<Bytes>> = HashMap::new();
    let avatar_list_raw = Arc::new(std::sync::Mutex::new(avatar_list_raw));
    {
        let event_sender = event_sender.clone();
        let state = state.clone();
        let avatar_list_raw = avatar_list_raw.clone();
        tokio::spawn(async move {
            loop {
                let state = state.lock().unwrap().clone();
                for (_id, user) in state.users {
                    match user.avatar {
                        Some(avatar) => {
                            if !avatar_list_raw.lock().unwrap().contains_key(&avatar) {
                                avatar_list_raw.lock().unwrap().insert(avatar.clone(), None);
                                println!("Requesting {}/{}", user.id, avatar);
                                let url = format!(
                                    "https://cdn.discordapp.com/avatars/{}/{}.png",
                                    user.id, avatar
                                );
                                match reqwest::Client::new()
                                    .get(url)
                                    .header(
                                        "Referer",
                                        "https://streamkit.discord.com/overlay/voice",
                                    )
                                    .header("User-Agent", "Mozilla/5.0")
                                    .send()
                                    .await
                                {
                                    Ok(resp) => match resp.bytes().await {
                                        Ok(bytes) => {
                                            avatar_list_raw
                                                .lock()
                                                .unwrap()
                                                .insert(user.id, Some(bytes));
                                            match event_sender
                                                .lock()
                                                .await
                                                .try_send("avatar".to_string())
                                            {
                                                Ok(_) => {}
                                                Err(_e) => {
                                                    println!("Unable to send to gtk thread {}", _e);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            println!("{}", err);
                                        }
                                    },
                                    Err(err) => {
                                        println!("{}", err);
                                    }
                                }
                            }
                        }
                        None => {}
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }
        });
    }

    // GTK/ Glib Main

    // avatar surfaces
    let avatar_list: HashMap<String, Option<ImageSurface>> = HashMap::new();
    let avatar_list = Arc::new(std::sync::Mutex::new(avatar_list));

    fn draw_deaf(ctx: &Context, pos_x: f64, pos_y: f64, size: f64) {
        ctx.save().expect("Could not save cairo state");
        ctx.translate(pos_x, pos_y);
        ctx.scale(size, size);
        ctx.set_source_rgba(1.0, 0.0, 0.0, 1.0);

        ctx.save().expect("Could not save cairo state");

        // Clip Strike-through
        ctx.set_fill_rule(FillRule::EvenOdd);
        ctx.set_line_width(0.1);
        ctx.move_to(0.0, 0.0);
        ctx.line_to(1.0, 0.0);
        ctx.line_to(1.0, 1.0);
        ctx.line_to(0.0, 1.0);
        ctx.line_to(0.0, 0.0);
        ctx.close_path();
        ctx.new_sub_path();
        ctx.arc(0.9, 0.1, 0.05, 1.25 * PI, 2.25 * PI);
        ctx.arc(0.1, 0.9, 0.05, 0.25 * PI, 1.25 * PI);
        ctx.close_path();
        ctx.clip();

        // Top band
        ctx.arc(0.5, 0.5, 0.2, 1.0 * PI, 0.0);
        ctx.stroke().expect("Could not stroke");

        // Left band
        ctx.arc(0.28, 0.65, 0.075, 1.5 * PI, 0.5 * PI);
        ctx.move_to(0.3, 0.5);
        ctx.line_to(0.3, 0.75);
        ctx.stroke().expect("Could not stroke");

        // Right band
        ctx.arc(0.72, 0.65, 0.075, 0.5 * PI, 1.5 * PI);
        ctx.move_to(0.7, 0.5);
        ctx.line_to(0.7, 0.75);
        ctx.stroke().expect("Could not stroke");

        ctx.restore().expect("Could not restore cairo state");
        // Strike through
        ctx.arc(0.7, 0.3, 0.035, 1.25 * PI, 2.25 * PI);
        ctx.arc(0.3, 0.7, 0.035, 0.25 * PI, 1.25 * PI);
        ctx.close_path();
        ctx.fill().expect("Could not fill");

        ctx.restore().expect("Could not restore");
    }

    fn draw_mute(ctx: &Context, pos_x: f64, pos_y: f64, size: f64) {
        ctx.save().expect("Could not save cairo state");
        ctx.translate(pos_x, pos_y);
        ctx.scale(size, size);
        ctx.set_source_rgba(1.0, 0.0, 0.0, 1.0);
        ctx.save().expect("Could not save cairo state");
        // Clip Strike-through
        ctx.set_fill_rule(FillRule::EvenOdd);
        ctx.set_line_width(0.1);
        ctx.move_to(0.0, 0.0);
        ctx.line_to(1.0, 0.0);
        ctx.line_to(1.0, 1.0);
        ctx.line_to(0.0, 1.0);
        ctx.line_to(0.0, 0.0);
        ctx.close_path();
        ctx.new_sub_path();
        ctx.arc(0.9, 0.1, 0.05, 1.25 * PI, 2.25 * PI);
        ctx.arc(0.1, 0.9, 0.05, 0.25 * PI, 1.25 * PI);
        ctx.close_path();
        ctx.clip();
        // Center
        ctx.set_line_width(0.07);
        ctx.arc(0.5, 0.3, 0.1, PI, 2.0 * PI);
        ctx.arc(0.5, 0.5, 0.1, 0.0, PI);
        ctx.close_path();
        ctx.fill().expect("Could not fill");
        ctx.set_line_width(0.05);
        // Stand rounded
        ctx.arc(0.5, 0.5, 0.15, 0.0, 1.0 * PI);
        ctx.stroke().expect("Could not stroke");
        // Stand vertical
        ctx.move_to(0.5, 0.65);
        ctx.line_to(0.5, 0.75);
        ctx.stroke().expect("Could not stroke");
        // Stand horizontal
        ctx.move_to(0.35, 0.75);
        ctx.line_to(0.65, 0.75);
        ctx.stroke().expect("Could not stroke");
        ctx.restore().expect("Coult not restore cairo state");
        // Strike through
        ctx.arc(0.7, 0.3, 0.035, 1.25 * PI, 2.25 * PI);
        ctx.arc(0.3, 0.7, 0.035, 0.25 * PI, 1.25 * PI);
        ctx.close_path();
        ctx.fill().expect("Could not fill");
        ctx.restore().expect("Could not restore cairo state");
    }

    fn set_untouchable(window: &gtk::ApplicationWindow) {
        let reg = Region::create();
        window.input_shape_combine_region(Some(&reg));
        window.set_accept_focus(false);
    }
    let application = gtk::Application::new(
        Some("io.github.trigg.discern"),
        gio::ApplicationFlags::REPLACE,
    );
    application.connect_activate(move |application: &gtk::Application| {
        // Create overlay
        let window = gtk::ApplicationWindow::new(application);

        // Customise redraw
        {
            let state = state.clone();
            let avatar_list = avatar_list.clone();
            let avatar_list_raw = avatar_list_raw.clone();
            window.connect_draw(move |window: &gtk::ApplicationWindow, ctx: &Context| {
                draw_overlay_gtk!(window, ctx, avatar_list, avatar_list_raw, state);

                Inhibit(false)
            });
        }

        // Set untouchable
        set_untouchable(&window);

        // Set as shell component
        gtk_layer_shell::init_for_window(&window);
        gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Bottom, true);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Left, true);
        gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Right, true);
        // Now we start!
        window.set_app_paintable(true);
        window.show_all();
        let state = state.clone();
        let gui_state = gui_state.clone();
        let future = {
            let event_recv = event_recv.clone();
            async move {
                while let Some(event) = event_recv.lock().await.next().await {
                    // We've just been alerted the state may have changed, we have a futures Mutex which can't be used in drawing, so copy data out to 'local' mutex!
                    let update_state: ConnState = gui_state.lock().await.clone();
                    let last_state: ConnState = state.lock().unwrap().clone();
                    if calculate_hash(&update_state) != calculate_hash(&last_state)
                        || event == "avatar"
                    {
                        state.lock().unwrap().replace_self(update_state);
                        window.queue_draw();
                    }
                }
            }
        };

        glib::MainContext::default().spawn_local(future);
    });
    let a: [String; 0] = Default::default(); // No args
    application.run_with_args(&a);
}
