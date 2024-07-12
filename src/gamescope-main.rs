extern crate cairo;
extern crate cairo_sys;
extern crate clap;
extern crate serde_json;
extern crate xcb;

use cairo::{Antialias, Context, FillRule, FontSlant, FontWeight, ImageSurface, Operator};
use cairorender::DiscordAvatarRaw;
use data::ConnState;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use futures_util::SinkExt;
use std::collections::hash_map::HashMap;
use std::f64::consts::PI;
use std::io::Cursor;
use std::sync::Arc;
use tokio::select;
use xcb::randr::Event::ScreenChangeNotify;
use xcb::{x, Xid};

mod cairorender;
mod core;
mod data;
mod macros;

#[tokio::main]
async fn main() {
    // Avatar to main thread
    let (mut avatar_request_sender, avatar_request_recv) =
        futures::channel::mpsc::channel::<ConnState>(10);

    // Mainthread to Avatar
    let (avatar_done_sender, mut avatar_done_recv) =
        futures::channel::mpsc::channel::<DiscordAvatarRaw>(10);

    // Websocket events to main thread
    let (event_sender, mut event_recv) = futures::channel::mpsc::channel::<ConnState>(10);
    let event_sender: Arc<Mutex<futures_channel::mpsc::Sender<ConnState>>> =
        Arc::new(Mutex::new(event_sender));

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

    let (conn, screen_num) =
        xcb::Connection::connect_with_extensions(None, &[xcb::Extension::RandR], &[]).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    let randr_data = xcb::randr::get_extension_data(&conn).expect("No RANDR");

    println!("{:?}", randr_data);
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
    let win = conn.generate_id();
    conn.flush().expect("Error on flush");

    // Create Window
    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: 32,
        wid: win,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: 1280,
        height: 720,
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
    conn.flush().expect("Error on flush");
    conn.check_request(cookie).expect("Error creating Window");
    conn.flush().expect("Error on flush");

    let mut window_width = 100; // Replaced on first Configure
    let mut window_height = 100;

    // Request XRandR
    let randr_cookie = conn.send_request(&xcb::randr::GetScreenResources { window: win });
    let randr_reply = conn
        .wait_for_reply(randr_cookie)
        .expect("Unable to get displays");

    for &crtc in randr_reply.crtcs() {
        let crtc_cookie = conn.send_request(&xcb::randr::GetCrtcInfo {
            crtc,
            config_timestamp: Default::default(),
        });
        match conn.wait_for_reply(crtc_cookie) {
            Ok(display) => {
                if window_width < display.width() || window_height < display.height() {
                    window_height = display.height();
                    window_width = display.width();
                }
            }
            Err(_) => {}
        };
    }

    // Request to be told of output changes
    let _callback_cookie = conn.send_request(&xcb::randr::SelectInput {
        window: win,
        enable: xcb::randr::NotifyMask::SCREEN_CHANGE,
    });
    conn.flush().expect("Error on flush");

    println!(
        "Starting window size : {} x {}",
        window_width, window_height
    );

    conn.send_request(&x::ConfigureWindow {
        window: win,
        value_list: &[
            x::ConfigWindow::Width(window_width as u32),
            x::ConfigWindow::Height(window_height as u32),
        ],
    });

    // Show window
    conn.send_request(&x::MapWindow { window: win });

    conn.flush().expect("Error on flush");

    // Prepare atoms
    let atom_overlay = {
        let cookies = (conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"GAMESCOPE_EXTERNAL_OVERLAY",
        }),);
        conn.wait_for_reply(cookies.0).unwrap().atom()
    };
    let mut state = ConnState::new();
    loop {
        let xloop = async { conn.poll_for_event() };
        let (xevent, threadevent, avatarevent) = select! {
            x = xloop => ( Some(x), None, None),
            x = event_recv.next() => (None,Some(x),None),
            x = avatar_done_recv.next() => (None, None, Some(x)),
        };
        let mut sleep = 100;
        let mut redraw = false;
        match xevent {
            Some(event) => match event {
                Ok(Some(event)) => match event {
                    xcb::Event::X(x::Event::Expose(_ev)) => {
                        redraw = true;
                        sleep = 0;
                    }
                    xcb::Event::X(x::Event::ClientMessage(_ev)) => {}
                    xcb::Event::X(x::Event::ConfigureNotify(ev)) => {
                        window_width = ev.width();
                        window_height = ev.height();
                        println!("Resized to {} x {}", window_width, window_height);
                        redraw = true;
                        sleep = 0;
                    }
                    xcb::Event::RandR(ScreenChangeNotify(ev)) => {
                        println!("Screen change : {} {}", ev.width(), ev.height());
                        if ev.width() > 1 && ev.height() > 1 {
                            conn.send_request(&x::ConfigureWindow {
                                window: win,
                                value_list: &[
                                    x::ConfigWindow::Width(ev.width() as u32),
                                    x::ConfigWindow::Height(ev.height() as u32),
                                ],
                            });
                        }
                    }
                    _ => {}
                },
                Ok(None) => {}
                Err(e) => {
                    println!("XCB Error : {:?}", e)
                }
            },
            None => {}
        }
        match threadevent {
            Some(event) => match event {
                Some(new_state) => {
                    state.replace_self(new_state.clone());
                    match avatar_request_sender.send(new_state.clone()).await {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Could not send state to avatar thread : {:?}", e)
                        }
                    }
                    redraw = true;
                    sleep = 0;
                }
                None => {}
            },
            None => {}
        }
        match avatarevent {
            Some(Some(avatardata)) => {
                match avatardata.raw {
                    Some(raw) => {
                        let surface = ImageSurface::create_from_png(&mut Cursor::new(raw))
                            .expect("Error processing user avatar");
                        avatar_list
                            .lock()
                            .unwrap()
                            .insert(avatardata.key.clone(), Some(surface));
                    }
                    None => {
                        println!("Raw is None for user id {}", avatardata.key);
                        avatar_list
                            .lock()
                            .unwrap()
                            .insert(avatardata.key.clone(), None);
                    }
                }

                redraw = true;
                sleep = 0;
            }
            Some(None) => {}
            None => {}
        }
        if redraw {
            let cr = create_cairo_context(&conn, &screen, &win, window_width, window_height);

            let should_show = state.users.len() > 0;
            set_as_overlay(&conn, &win, &atom_overlay, should_show);
            draw_overlay!(&cr, avatar_list, state);
        }
        if sleep > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
        }
        conn.flush().expect("Flush error");
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
