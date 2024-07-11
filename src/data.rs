#[cfg(feature = "state_hash")]
use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Hash)]
pub struct DiscordUserData {
    pub avatar: Option<String>,
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, Hash)]
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

#[cfg(feature = "state_hash")]
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl Hash for ConnState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.user_id.hash(state);
        self.voice_channel.hash(state);
        for (id, user) in self.users.clone() {
            id.hash(state);
            user.hash(state);
        }
        for (id, voice_state) in self.voice_states.clone() {
            id.hash(state);
            voice_state.hash(state);
        }
    }
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

    pub fn clear(&mut self) {
        self.user_id = None;
        self.voice_channel = None;
        self.users.clear();
        self.voice_states.clear();
    }
}
