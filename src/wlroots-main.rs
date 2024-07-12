extern crate clap;
extern crate serde_json;
use crate::data::calculate_hash;
use crate::data::ConnState;
use cairo::{
    Antialias, Context, FillRule, FontSlant, FontWeight, ImageSurface, Operator, RectangleInt,
    Region,
};
use cairorender::DiscordAvatarRaw;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use futures_util::SinkExt;
use gio::prelude::*;
use glib;
use gtk::prelude::*;
use gtk_layer_shell;
use std::collections::hash_map::HashMap;
use std::f64::consts::PI;
use std::io::Cursor;
use std::sync::Arc;

mod cairorender;
mod core;
mod data;
mod macros;

#[tokio::main]
async fn main() {
    // Avatar to main thread
    let (avatar_request_sender, avatar_request_recv) =
        futures::channel::mpsc::channel::<ConnState>(10);
    let avatar_request_sender = Arc::new(Mutex::new(avatar_request_sender));

    // Mainthread to Avatar
    let (avatar_done_sender, avatar_done_recv) =
        futures::channel::mpsc::channel::<DiscordAvatarRaw>(10);

    let avatar_done_recv = Arc::new(Mutex::new(avatar_done_recv));

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

    // Start a thread for avatars
    cairorender::avatar_downloader(avatar_done_sender, avatar_request_recv).await;

    // Avatar grabbing thread
    let state = Arc::new(std::sync::Mutex::new(ConnState::new()));

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
            window.connect_draw(move |window: &gtk::ApplicationWindow, ctx: &Context| {
                draw_overlay_gtk!(window, ctx, avatar_list, state);

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

        // State watcher
        glib::MainContext::default().spawn_local({
            let window = window.clone();
            let event_recv = event_recv.clone();
            let avatar_request_sender = avatar_request_sender.clone();
            async move {
                while let Some(event) = event_recv.lock().await.next().await {
                    // We've just been alerted the state may have changed, we have a futures Mutex which can't be used in drawing, so copy data out to 'local' mutex!
                    let update_state: ConnState = event.clone();
                    let last_state: ConnState = state.lock().unwrap().clone();
                    let _ = avatar_request_sender.lock().await.send(event.clone()).await;
                    if calculate_hash(&update_state) != calculate_hash(&last_state) {
                        state.lock().unwrap().replace_self(update_state);
                        window.queue_draw();
                    }
                }
            }
        });

        // Avatar watcher
        glib::MainContext::default().spawn_local({
            let window = window.clone();
            let avatar_done_recv = avatar_done_recv.clone();
            let avatar_list = avatar_list.clone();
            async move {
                while let Some(event) = avatar_done_recv.lock().await.next().await {
                    match event.raw {
                        Some(raw) => {
                            let surface = ImageSurface::create_from_png(&mut Cursor::new(raw))
                                .expect("Error processing user avatar");
                            avatar_list
                                .lock()
                                .unwrap()
                                .insert(event.key.clone(), Some(surface));
                        }
                        None => {
                            println!("Raw is None for user id {}", event.key);
                            avatar_list.lock().unwrap().insert(event.key.clone(), None);
                        }
                    }
                    window.queue_draw();
                }
            }
        });
    });
    let a: [String; 0] = Default::default(); // No args
    application.run_with_args(&a);
}
