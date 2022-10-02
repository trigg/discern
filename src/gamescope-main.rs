extern crate cairo;
extern crate cairo_sys;
extern crate clap;
extern crate serde_json;
extern crate xcb;

use xcb::{x, Xid};

use bytes::Bytes;
use cairo::{Antialias, Context, FillRule, FontSlant, FontWeight, ImageSurface, Operator};
use futures::lock::Mutex;
use futures::stream::StreamExt;
use std::collections::hash_map::HashMap;
use std::f64::consts::PI;
use std::io::Cursor;
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};

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
    let avatar_list_raw: HashMap<String, Option<Bytes>> = HashMap::new();
    let avatar_list_raw = Arc::new(std::sync::Mutex::new(avatar_list_raw));
    {
        let event_sender = event_sender.clone();
        let state = state.clone();
        let avatar_list_raw = avatar_list_raw.clone();
        tokio::spawn(async move {
            println!("Starting avatar thread");
            loop {
                let state = state.lock().await.clone();
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
                                                    println!(
                                                        "Unable to send to main thread {}",
                                                        _e
                                                    );
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

    // XCB Main

    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    // Find RGBA visual
    let visualid = transparent_visual(screen).unwrap().visual_id();
    // Create Colormap
    let colormap = conn.generate_id();
    let cookie = conn.send_request_checked(&x::CreateColormap {
        alloc: x::ColormapAlloc::None,
        mid: colormap,
        window: screen.root(),
        visual: visualid,
    });
    conn.check_request(cookie).expect("Error creating ColorMap");
    conn.flush().expect("Error on flush");

    // Create Window
    let win = conn.generate_id();
    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: 32,
        wid: win,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: 1280,
        height: 800,
        border_width: 1,
        class: x::WindowClass::InputOutput,
        visual: visualid,
        // this list must be in same order than `Cw` enum order
        value_list: &[
            x::Cw::BackPixel(0),
            x::Cw::BorderPixel(0),
            x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::STRUCTURE_NOTIFY),
            x::Cw::Colormap(colormap),
        ],
    });
    conn.check_request(cookie).expect("Error creating Window");
    conn.send_request(&x::MapWindow { window: win });

    conn.flush().expect("Error on flush");

    let mut window_width = 100; // Replaced on first Configure
    let mut window_height = 100;

    // Prepare atoms
    let atom_overlay = {
        let cookies = (conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"GAMESCOPE_EXTERNAL_OVERLAY",
        }),);
        conn.wait_for_reply(cookies.0).unwrap().atom()
    };

    loop {
        // TODO Can we use a switch on conn.poll_for_event and event_recv....try_next?
        tokio::time::sleep(Duration::from_millis(1000 / 60)).await;
        let event = conn.poll_for_event();
        match event {
            Err(err) => {
                println!("{}", err);
                break;
            }
            Ok(event) => {
                match event {
                    Some(event) => match event {
                        xcb::Event::X(x::Event::Expose(_ev)) => {
                            let cr = create_cairo_context(
                                &conn,
                                &screen,
                                &win,
                                window_width,
                                window_height,
                            );
                            draw_overlay!(&cr, avatar_list, avatar_list_raw, gui_state.clone());
                        }
                        xcb::Event::X(x::Event::ClientMessage(_ev)) => {}
                        xcb::Event::X(x::Event::ConfigureNotify(ev)) => {
                            window_width = ev.width();
                            window_height = ev.height();
                            let cr = create_cairo_context(
                                &conn,
                                &screen,
                                &win,
                                window_width,
                                window_height,
                            );
                            draw_overlay!(&cr, avatar_list, avatar_list_raw, gui_state.clone());
                        }
                        _ => {}
                    },
                    None => {
                        // Check if we've got any new data
                        let a = timeout(Duration::ZERO, event_recv.lock().await.next()).await;
                        match a {
                            Ok(Some(_t)) => {
                                // Drain the list
                                while let Ok(Some(_event)) =
                                    timeout(Duration::ZERO, event_recv.lock().await.next()).await
                                {
                                }
                                let cr = create_cairo_context(
                                    &conn,
                                    &screen,
                                    &win,
                                    window_width,
                                    window_height,
                                );
                                draw_overlay!(&cr, avatar_list, avatar_list_raw, gui_state.clone());

                                // Replace overlay status
                                // TODO use more than chat room user list to decide
                                let state = gui_state.lock().await.clone();

                                let should_show = state.users.len() > 0;

                                set_as_overlay(&conn, &win, &atom_overlay, should_show);
                            }
                            Ok(None) => {}
                            Err(_) => {}
                        }
                    }
                }
                conn.flush().expect("Flush error");
            }
        }
    }
}

fn create_cairo_context(
    conn: &xcb::Connection,
    screen: &x::Screen,
    window: &x::Window,
    window_width: u16,
    window_height: u16,
) -> cairo::Context {
    let surface;
    unsafe {
        let cairo_conn = cairo::XCBConnection::from_raw_none(
            conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t
        );
        let mut visualtype = transparent_visual(screen).unwrap();
        let visual_ptr: *mut cairo_sys::xcb_visualtype_t =
            &mut visualtype as *mut _ as *mut cairo_sys::xcb_visualtype_t;
        let visual = cairo::XCBVisualType::from_raw_none(visual_ptr);
        let cairo_screen = cairo::XCBDrawable(window.resource_id());
        surface = cairo::XCBSurface::create(
            &cairo_conn,
            &cairo_screen,
            &visual,
            window_width as i32,
            window_height as i32,
        )
        .unwrap();
    }

    cairo::Context::new(&surface).unwrap()
}

fn transparent_visual(screen: &x::Screen) -> Option<x::Visualtype> {
    for depth in screen.allowed_depths() {
        if depth.depth() == 32 {
            for visual in depth.visuals() {
                if visual.class() == xcb::x::VisualClass::TrueColor {
                    return Some(*visual);
                }
            }
        }
    }
    None
}

fn set_as_overlay(conn: &xcb::Connection, win: &x::Window, atom: &x::Atom, enabled: bool) {
    let enabled: u32 = match enabled {
        false => 0,
        true => 1,
    };
    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: *win,
        property: *atom,
        r#type: x::ATOM_CARDINAL,
        data: &[enabled as u32],
    });
    conn.flush().expect("Error on flush");
}
