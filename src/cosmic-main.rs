extern crate clap;
extern crate serde_json;
use crate::data::ConnState;
use cairorender::DiscordAvatarRaw;
use cosmic::iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings;
use cosmic::iced::wayland::actions::window::SctkWindowSettings;
use cosmic::iced::widget::{column, container, image, row, text};
use cosmic::iced::window::Id;
use cosmic::iced::{theme::Palette, Color, Theme};
use cosmic::iced::{Application, Element, Length};
use cosmic::{iced, iced::Subscription};
use data::calculate_hash;
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

pub enum Location {
    Left,
    Right,
}

pub struct Preferences {
    location: Location,
}

pub struct App {
    height: f32,
    preferences: Preferences,
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

struct NormalStyle;
struct TalkingStyle;
struct MuteStyle;

struct NormalImageStyle;
struct TalkingImageStyle;
struct MuteImageStyle;

impl iced::widget::container::StyleSheet for NormalStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let mut border = iced::Border::with_radius(5.0);
        border.color = iced::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            })),
            text_color: Some(Color::WHITE),
            border: border,
            ..Default::default()
        }
    }
}
impl iced::widget::container::StyleSheet for TalkingStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let mut border = iced::Border::with_radius(5.0);
        border.color = iced::Color {
            r: 0.0,
            g: 0.3,
            b: 0.0,
            a: 0.6,
        };
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.3,
                b: 0.0,
                a: 1.0,
            })),
            text_color: Some(Color::WHITE),
            border: border,
            ..Default::default()
        }
    }
}
impl iced::widget::container::StyleSheet for MuteStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let mut border = iced::Border::with_radius(5.0);
        border.color = iced::Color {
            r: 0.3,
            g: 0.0,
            b: 0.0,
            a: 0.6,
        };
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.3,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            })),
            text_color: Some(Color::WHITE),
            border: border,
            ..Default::default()
        }
    }
}

impl iced::widget::container::StyleSheet for NormalImageStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let mut border = iced::Border::with_radius(32.0);
        border.color = iced::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
        border.width = 10.0;
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            })),
            text_color: Some(Color::WHITE),
            border: border,
            ..Default::default()
        }
    }
}
impl iced::widget::container::StyleSheet for TalkingImageStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let mut border = iced::Border::with_radius(32.0);
        border.color = iced::Color {
            r: 0.0,
            g: 0.3,
            b: 0.0,
            a: 0.6,
        };
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.3,
                b: 0.0,
                a: 1.0,
            })),
            text_color: Some(Color::WHITE),
            border: border,
            ..Default::default()
        }
    }
}
impl iced::widget::container::StyleSheet for MuteImageStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let mut border = iced::Border::with_radius(32.0);
        border.color = iced::Color {
            r: 0.3,
            g: 0.0,
            b: 0.0,
            a: 0.6,
        };
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.3,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            })),
            text_color: Some(Color::WHITE),
            border: border,
            ..Default::default()
        }
    }
}

impl Application for App {
    type Executor = iced::executor::Default;

    type Flags = UiFlags;

    type Message = Message;

    type Theme = Theme;

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::StateRecv(msg) => {
                if calculate_hash(&msg) != calculate_hash(&self.state) {
                    self.state = msg.clone();
                    match self.send_avatar.lock().unwrap().try_send(msg.clone()) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("Unable to send state to avatar thread: {}", err);
                        }
                    }
                }
                return iced::Command::none();
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
        let height = (self.state.users.len() as f32) * 64.0;
        if self.height != height {
            println!("Resizing {} >  {}", self.height, height);
            self.height = height;
            iced::window::resize(
                iced::window::Id::MAIN,
                iced::Size {
                    width: 200.0,
                    height: height,
                },
            )
        } else {
            println!("No resize {}", height);
            iced::Command::none()
        }
    }

    /// Creates a view after each update.
    fn view(&self, _id: Id) -> Element<Self::Message> {
        println!("Rerender");
        let mut window_container = column([]);

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

                let inner_image = Element::from(
                    image::Image::<image::Handle>::new(image_handle)
                        .border_radius([32.0, 32.0, 32.0, 32.0])
                        .width(Length::Fixed(64.0))
                        .height(Length::Fixed(64.0)),
                );
                let image = container(inner_image).style(if voice_data.talking {
                    iced::theme::Container::Custom(Box::new(TalkingImageStyle))
                } else {
                    if voice_data.mute
                        || voice_data.deaf
                        || voice_data.self_deaf
                        || voice_data.self_mute
                    {
                        iced::theme::Container::Custom(Box::new(MuteImageStyle))
                    } else {
                        iced::theme::Container::Custom(Box::new(NormalImageStyle))
                    }
                });
                let text = container(
                    container(text(
                        voice_data.nick.clone().unwrap_or(value.username.clone()),
                    ))
                    .padding(4)
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .style(if voice_data.talking {
                        iced::theme::Container::Custom(Box::new(TalkingStyle))
                    } else {
                        if voice_data.mute
                            || voice_data.deaf
                            || voice_data.self_mute
                            || voice_data.self_deaf
                        {
                            iced::theme::Container::Custom(Box::new(MuteStyle))
                        } else {
                            iced::theme::Container::Custom(Box::new(NormalStyle))
                        }
                    }),
                )
                .width(Length::Fill)
                .height(Length::Fixed(64.0))
                .center_y()
                .align_x(match self.preferences.location {
                    Location::Left => iced::alignment::Horizontal::Left,
                    Location::Right => iced::alignment::Horizontal::Right,
                });

                let row = match self.preferences.location {
                    Location::Left => row([Element::from(image), Element::from(text)]),
                    Location::Right => row([Element::from(text), Element::from(image)]),
                };
                let row_cont = container(row)
                    .height(Length::Shrink)
                    .width(Length::Shrink)
                    .height(Length::Shrink);
                window_container = window_container.push(row_cont);
            }
        }

        Element::from(window_container)
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
                height: 0f32,
                preferences: Preferences {
                    location: Location::Right,
                },
                state: ConnState::new(),
                recv_state: RefCell::new(Some(input.recv_state)),
                recv_avatar: RefCell::new(Some(input.recv_avatar)),
                send_avatar: Arc::new(std::sync::Mutex::new(input.send_avatar)),
                avatar_handler: Arc::new(std::sync::Mutex::new(HashMap::new())),
            },
            iced::Command::none(),
        )
    }

    fn title(&self, _id: Id) -> String {
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
        futures::channel::mpsc::channel::<ConnState>(10);

    // Mainthread to Avatar
    let (avatar_done_sender, avatar_done_recv) =
        futures::channel::mpsc::channel::<DiscordAvatarRaw>(10);

    cairorender::avatar_downloader(avatar_done_sender, avatar_request_recv).await;

    // Websocket events to main thread
    let (event_sender, event_recv) = futures::channel::mpsc::channel::<ConnState>(10);
    let event_sender = Arc::new(Mutex::new(event_sender));

    // Main thread messages to Websocket output
    let (msg_sender, msg_recv) = futures::channel::mpsc::channel::<String>(10);
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

    let debug = false;

    let initial_window = if debug {
        InitialSurface::XdgWindow(SctkWindowSettings {
            window_id: Id::MAIN,
            app_id: Some("Discern".to_string()),
            title: Some("Discern".to_string()),
            parent: None,
            autosize: false,
            resizable: None,
            client_decorations: true,
            transparent: false,
            ..Default::default()
        })
    } else {
        InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
            id: Id::MAIN,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "Discern".into(),
            layer: Layer::Overlay,
            size: Some((Some(200), Some(200))),
            anchor: Anchor::RIGHT.union(Anchor::TOP),
            exclusive_zone: 0 as i32,
            ..Default::default()
        })
    };
    /*
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
    */
    let settings = iced::Settings {
        id: None,
        flags: input,
        fonts: Default::default(),
        default_font: Default::default(),
        default_text_size: 14.into(),
        antialiasing: true,
        exit_on_close_request: true,
        initial_surface: initial_window,
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
