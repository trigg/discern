use crate::data::ConnState;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use gio::prelude::*;
use glib;
use gtk::cairo::{RectangleInt, Antialias, Context, FillRule, FontSlant, FontWeight, Operator, Region};
use gtk::prelude::*;
use std::f64::consts::PI;
use std::sync::Arc;

pub fn start(gui_state: Arc<Mutex<ConnState>>) -> impl Fn(String) {
    let (event_sender, event_recv) = futures::channel::mpsc::channel(0); // Pipe to send notification that state has changed
    let event_sender = Arc::new(std::sync::Mutex::new(event_sender));
    let event_recv = Arc::new(std::sync::Mutex::new(event_recv));
    let state = Arc::new(std::sync::Mutex::new(ConnState::new())); // 'local' mutex copy, not shared with main
    let _gui_loop = tokio::task::spawn(async move {


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

            let reg = Region::create();
            reg.union_rectangle(& RectangleInt{
                x: 0,
                y: 0,
                width:1,
                height:1
            }).expect("Failed to add rectangle"); 
            // If region ends up as empty at this point then queue_draw is ignored and we never draw again!
            window.shape_combine_region(Some(&reg));
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
                window.connect_draw(move |window: &gtk::ApplicationWindow, ctx: &Context| {
                    // Set XShape
                    draw_overlay!(window,ctx, state);
                    Inhibit(false)
                });
            }

            // Set untouchable
            set_untouchable(&window);

            // Now we start!
            window.set_app_paintable(true);
            window.set_skip_pager_hint(true);
            window.set_skip_taskbar_hint(true);
            window.set_keep_above(true);
            window.set_decorated(false);
            window.set_accept_focus(false);
            window.maximize();

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