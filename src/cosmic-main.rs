extern crate clap;
extern crate serde_json;
use crate::data::ConnState;
use cairorender::DiscordAvatarRaw;
use cosmic::iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings;
use cosmic::iced::widget::{column, image, row, text};
use cosmic::iced::window::Id;
use cosmic::iced::{theme::Palette, Color, Theme};
use cosmic::iced::{Application, Element};
use cosmic::{iced, iced::Subscription};
use futures::lock::Mutex;
use futures::stream::StreamExt;
use futures_channel::mpsc;
use iced_sctk::commands::layer_surface::{Anchor, KeyboardInteractivity, Layer};
use iced_sctk::settings::InitialSurface;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

mod cairorender;
mod core;
mod data;
mod macros;

pub struct App {
    state: ConnState,
    recv_state: RefCell<Option<mpsc::Receiver<ConnState>>>,
    recv_avatar: RefCell<Option<mpsc::Receiver<DiscordAvatarRaw>>>,
    send_avatar: Arc<std::sync::Mutex<mpsc::Sender<ConnState>>>,
    avatar_handler: Arc<std::sync::Mutex<HashMap<String, image::Handle>>>,
}

pub struct UiFlags {
    recv_state: mpsc::Receiver<ConnState>,
    recv_avatar: mpsc::Receiver<DiscordAvatarRaw>,
    send_avatar: mpsc::Sender<ConnState>,
}

/// Messages that are used specifically by our [`App`].
#[derive(Clone, Debug)]
pub enum Message {
    StateRecv(ConnState),
    AvatarRecv(DiscordAvatarRaw),
}

impl Application for App {
    type Executor = iced::executor::Default;

    type Flags = UiFlags;

    type Message = Message;

    type Theme = Theme;

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::StateRecv(msg) => {
                self.state = msg.clone();
                match self.send_avatar.lock().unwrap().try_send(msg.clone()) {
                    Ok(_) => {}
                    Err(err) => {
                        println!("Unable to send state to avatar thread: {}", err);
                    }
                }
            }
            Message::AvatarRecv(msg) => {
                match msg.raw {
                    Some(bytes) => {
                        let byte_image: image::Handle = image::Handle::from_memory(bytes);
                        self.avatar_handler
                            .lock()
                            .unwrap()
                            .insert(msg.key, byte_image);
                    }
                    None => {}
                };
            }
        }
        iced::Command::none()
    }

    /// Creates a view after each update.
    fn view(&self, id: Id) -> Element<Self::Message> {
        let mut container = column([]);

        for (id, value) in self.state.users.iter() {
            let value = value.clone();
            if let Some(voice_data) = self.state.voice_states.get(id) {
                let avatar_key = format!("{}/{}", id, value.avatar.unwrap());

                let image_handle = match self.avatar_handler.lock().unwrap().get(&avatar_key) {
                    Some(handle) => handle.clone(),
                    None => {
                        image::Handle::from_path("/home/triggerhapp/.local/share/icons/hicolor/256x256/apps/discover-overlay-tray.png")
                    }
                };

                let row = row([
                    Element::from(image::Image::<image::Handle>::new(image_handle)),
                    Element::from(text(
                        voice_data.nick.clone().unwrap_or(value.username.clone()),
                    )),
                ]);

                container = container.push(row);
            }
        }

        Element::from(container)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::subscription::unfold(
                "connstate changes",
                self.recv_state.take(),
                move |mut receiver| async move {
                    let new_state = receiver.as_mut().unwrap().next().await.unwrap().clone();
                    (Message::StateRecv(new_state), receiver)
                },
            ),
            iced::subscription::unfold(
                "avatar changes",
                self.recv_avatar.take(),
                move |mut receiver| async move {
                    let new_avatar_data = receiver.as_mut().unwrap().next().await.unwrap();
                    (Message::AvatarRecv(new_avatar_data), receiver)
                },
            ),
        ])
    }

    fn new(input: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            App {
                state: ConnState::new(),
                recv_state: RefCell::new(Some(input.recv_state)),
                recv_avatar: RefCell::new(Some(input.recv_avatar)),
                send_avatar: Arc::new(std::sync::Mutex::new(input.send_avatar)),
                avatar_handler: Arc::new(std::sync::Mutex::new(HashMap::new())),
            },
            iced::Command::none(),
        )
    }

    fn title(&self, id: Id) -> String {
        "Discern".into()
    }

    fn theme(&self, _id: Id) -> Self::Theme {
        discern_theme()
    }
}

#[tokio::main]
async fn main() {
    // Avatar to main thread
    let (avatar_request_sender, avatar_request_recv) =
        futures::channel::mpsc::channel::<ConnState>(0);

    // Mainthread to Avatar
    let (avatar_done_sender, avatar_done_recv) =
        futures::channel::mpsc::channel::<DiscordAvatarRaw>(0);

    cairorender::avatar_downloader(avatar_done_sender, avatar_request_recv).await;

    // Websocket events to main thread
    let (event_sender, event_recv) = futures::channel::mpsc::channel::<ConnState>(0);
    let event_sender = Arc::new(Mutex::new(event_sender));

    // Main thread messages to Websocket output
    let (msg_sender, msg_recv) = futures::channel::mpsc::channel::<String>(0);
    let _msg_sender = Arc::new(Mutex::new(msg_sender));
    let msg_recv = Arc::new(Mutex::new(msg_recv));

    // Start a thread for connection
    let connector_event_sender = event_sender.clone();
    let connector_msg_recv = msg_recv.clone();
    core::connector(connector_event_sender.clone(), connector_msg_recv.clone()).await;

    let input = UiFlags {
        recv_state: event_recv,
        recv_avatar: avatar_done_recv,
        send_avatar: avatar_request_sender,
    };
    let settings = iced::Settings {
        id: None,
        initial_surface: InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
            id: Id::MAIN,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "Discern".into(),
            layer: Layer::Overlay,
            size: Some((Some(200), Some(200))),
            anchor: Anchor::RIGHT.union(Anchor::TOP),
            exclusive_zone: 0 as i32,
            ..Default::default()
        }),
        flags: input,
        fonts: Default::default(),
        default_font: Default::default(),
        default_text_size: 14.into(),
        antialiasing: true,
        exit_on_close_request: true,
    };
    match App::run(settings) {
        Ok(_) => {}
        Err(err) => {
            println!("Error : {:?}", err);
        }
    }
}

pub fn discern_theme() -> Theme {
    Theme::custom(
        "discern".into(),
        Palette {
            background: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            text: Color::from_rgba(0.0, 0.0, 0.0, 1.0),
            primary: Color::from_rgba(0.1, 0.5, 0.1, 1.0),
            success: Color::from_rgba(0.0, 1.0, 0.0, 1.0),
            danger: Color::from_rgba(1.0, 0.0, 0.0, 1.0),
        },
    )
}
