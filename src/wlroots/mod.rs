use crate::data::ConnState;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use gio::prelude::*;
use glib;
use gtk::cairo::{Antialias, Context, FillRule, FontSlant, FontWeight, Operator, Region};
use gtk::prelude::*;
use gtk_layer_shell;
use std::f64::consts::PI;
use std::sync::Arc;

pub fn start(gui_state: Arc<Mutex<ConnState>>) -> impl Fn(String) {
    let (event_sender, event_recv) = futures::channel::mpsc::channel(0); // Pipe to send notification that state has changed
    let event_sender = Arc::new(std::sync::Mutex::new(event_sender));
    let event_recv = Arc::new(std::sync::Mutex::new(event_recv));
    let state = Arc::new(std::sync::Mutex::new(ConnState::new())); // 'local' mutex copy, not shared with main
    let _gui_loop = tokio::task::spawn(async move {
        // Config / Static
        let edge = 6.0;
        let line_height = 32.0;

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
                window.connect_draw(move |_window: &gtk::ApplicationWindow, ctx: &Context| {
                    ctx.set_antialias(Antialias::Good);
                    ctx.set_operator(Operator::Source);
                    ctx.set_source_rgba(1.0, 0.0, 0.0, 0.0);
                    ctx.paint().expect("Unable to paint window");

                    ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
                    ctx.set_font_size(16.0);
                    let state = state.lock().unwrap().clone();
                    let mut y = 50.0;
                    ctx.set_operator(Operator::Over);

                    if state.users.len() > 0 {
                        for (key, user) in state.users {
                            match state.voice_states.get(&key) {
                                Some(voice_state) => {
                                    let mut name = user.username.clone();
                                    match &voice_state.nick {
                                        Some(nick) => {
                                            name = nick.clone();
                                        }
                                        None => {}
                                    }
                                    if voice_state.talking {
                                        ctx.set_source_rgba(0.0, 0.4, 0.0, 0.6);
                                    } else {
                                        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.4);
                                    }
                                    let ext = ctx.text_extents(&name).unwrap();
                                    ctx.rectangle(
                                        line_height,
                                        y + (line_height / 2.0) - (ext.height / 2.0) - edge,
                                        ext.width + edge * 2.0,
                                        ext.height + edge * 2.0,
                                    );
                                    ctx.fill().expect("Unable to fill");
                                    ctx.move_to(
                                        line_height + edge,
                                        y + (line_height / 2.0) + (ext.height / 2.0),
                                    );

                                    if voice_state.talking {
                                        ctx.set_source_rgba(0.0, 1.0, 0.0, 1.0);
                                    } else {
                                        ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                                    }
                                    ctx.show_text(&name).expect("unable to draw text");
                                    if voice_state.deaf || voice_state.self_deaf {
                                        draw_deaf(ctx, 0.0, y, line_height);
                                    } else if voice_state.mute || voice_state.self_mute {
                                        draw_mute(ctx, 0.0, y, line_height);
                                    }
                                }
                                None => {}
                            }
                            y += line_height;
                        }
                    }
                    //draw_mute(ctx, 0.0, 0.0, 60.0);

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
                    while let Some(_event) = event_recv.lock().unwrap().next().await {
                        // TODO Maybe filter for events that actually require a redraw instead of redraw for everything?

                        // TODO Maybe just write a simple hasher for ConnData and compare hashes? Could avoid queue_draw if we know state is equal

                        // We've just been alerted the state may have changed, we have a futures Mutex which can't be used in drawing, so copy data out to 'local' mutex!
                        let update_state: ConnState = gui_state.lock().await.clone();
                        state.lock().unwrap().replace_self(update_state);
                        window.queue_draw();
                    }
                }
            };

            glib::MainContext::default().spawn_local(future);
        });
        let a: [String; 0] = Default::default(); // No args
        application.run_with_args(&a);
    });

    let event_sender = event_sender.clone();
    move |value: String| match event_sender.lock().unwrap().try_send(value) {
        Ok(_) => {}
        Err(_e) => {
            println!("Unable to send to gtk thread {}", _e);
        }
    }
}
