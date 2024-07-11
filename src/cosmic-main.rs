extern crate clap;
extern crate serde_json;
use crate::data::ConnState;
use cairorender::DiscordAvatarRaw;
use cosmic::cosmic_config::Config;
use cosmic::cosmic_theme::palette::Srgba;
use cosmic::cosmic_theme::ThemeBuilder;
use cosmic::widget::image;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use futures_channel::mpsc;
use std::cell::RefCell;

use std::collections::HashMap;
use std::sync::Arc;

use cosmic::app;
use cosmic::iced_core::Size;
use cosmic::{executor, iced, iced::Subscription, ApplicationExt, Element};

mod cairorender;
mod core;
mod data;
mod macros;

pub struct App {
    core: app::Core,
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

impl cosmic::Application for App {
    /// Default async executor to use with the app.
    type Executor = executor::Default;

    /// Argument received [`cosmic::Application::new`].
    type Flags = UiFlags;

    /// Message type specific to our [`App`].
    type Message = Message;

    /// The unique application ID to supply to the window manager.
    const APP_ID: &'static str = "io.github.trigg.discern.cosmic";

    fn core(&self) -> &app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut app::Core {
        &mut self.core
    }

    /// Creates the application, and optionally emits command on initialize.
    fn init(core: app::Core, input: Self::Flags) -> (Self, app::Command<Self::Message>) {
        let mut app = App {
            core,
            state: ConnState::new(),
            recv_state: RefCell::new(Some(input.recv_state)),
            recv_avatar: RefCell::new(Some(input.recv_avatar)),
            send_avatar: Arc::new(std::sync::Mutex::new(input.send_avatar)),
            avatar_handler: Arc::new(std::sync::Mutex::new(HashMap::new())),
        };

        let command = app.update_title(); // Command::none()

        (app, command)
    }

    fn update(&mut self, message: Message) -> app::Command<Message> {
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
        app::Command::none()
    }

    /// Creates a view after each update.
    fn view(&self) -> Element<Self::Message> {
        let mut container = cosmic::widget::column();

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

                let row = cosmic::widget::row::with_children(
                    [
                        Element::from(image::Image::<image::Handle>::new(image_handle)),
                        Element::from(cosmic::widget::text(
                            voice_data.nick.clone().unwrap_or(value.username.clone()),
                        )),
                    ]
                    .into(),
                );

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
    let config = Config::new("test", 0).unwrap();
    let mut theme = ThemeBuilder::dark();
    let _ = theme.set_bg_color(&config, Some(Srgba::new(0, 0, 0, 0).into()));
    let custom_theme = theme.build();
    let settings = app::Settings::default()
        .size(Size::new(1024., 768.))
        .transparent(true)
        .client_decorations(false)
        .theme(cosmic::Theme::custom(Arc::new(custom_theme)));

    let _ = app::run::<App>(settings, input);
}

impl App
where
    Self: cosmic::Application,
{
    fn update_title(&mut self) -> app::Command<Message> {
        self.set_window_title("Discern".to_string(), self.core.focused_window().unwrap())
    }
}
