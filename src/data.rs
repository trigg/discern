use std::collections::hash_map::HashMap;
use std::vec::Vec;

#[derive(Debug,  Clone)]
pub struct DiscordUserData {
    pub avatar: String,
    pub id: String,
    pub username: String
}

#[derive(Debug,  Clone)]
pub struct VoiceStateData {
    pub mute: bool,
    pub self_mute: bool,
    pub deaf: bool,
    pub self_deaf: bool,
    pub suppress: bool,
    pub nick: Option<String>,
    pub talking: bool
}

#[derive(Debug, Clone)]
pub struct ConnState {
    pub user_id: Option<String>,
    pub voice_channel: Option<String>,
    pub users: HashMap<String, DiscordUserData>,
    pub voice_states: HashMap<String, VoiceStateData>,
    pub errors: Vec<String>,
}
