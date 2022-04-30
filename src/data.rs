use std::collections::hash_map::HashMap;

#[derive(Debug, Clone)]
pub struct DiscordUserData {
    pub avatar: String,
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct VoiceStateData {
    pub mute: bool,
    pub self_mute: bool,
    pub deaf: bool,
    pub self_deaf: bool,
    pub suppress: bool,
    pub nick: Option<String>,
    pub talking: bool,
}

#[derive(Debug, Clone)]
pub struct ConnState {
    pub user_id: Option<String>,
    pub voice_channel: Option<String>,
    pub users: HashMap<String, DiscordUserData>,
    pub voice_states: HashMap<String, VoiceStateData>,
}

impl ConnState {
    pub fn new() -> ConnState {
        ConnState {
            user_id: None,
            voice_channel: None,
            users: HashMap::new(),
            voice_states: HashMap::new(),
        }
    }

    // Replace own contents with contents of 'new'
    pub fn replace_self(&mut self, new: ConnState) {
        self.user_id = new.user_id.clone();
        self.voice_channel = new.voice_channel.clone();
        self.users.clear();
        for (key, val) in new.users.iter() {
            self.users.insert(key.clone(), val.clone());
        }
        self.voice_states.clear();
        for (key, val) in new.voice_states.iter() {
            self.voice_states.insert(key.clone(), val.clone());
        }
    }
}
