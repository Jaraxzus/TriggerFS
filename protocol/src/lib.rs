use std::{
    fmt::{Debug, Display},
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

use elfo::prelude::*;
use fs::actions::Action;
use notify::Event;

#[message(part)]
pub struct KeyAction {
    pub path: PathBuf,
    pub action: Action,
}

impl PartialEq for KeyAction {
    fn eq(&self, other: &Self) -> bool {
        let mut self_hasher = DefaultHasher::new();
        self.hash(&mut self_hasher);
        let mut other_hasher = DefaultHasher::new();
        other.hash(&mut other_hasher);
        self_hasher.finish() == other_hasher.finish()
    }
}
impl Eq for KeyAction {}

impl Display for KeyAction {
    // отвечает за фомирование ключа для актора
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}_{:?})", self.path, self.action)
    }
}

impl Hash for KeyAction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{}", self).hash(state)
    }
}

#[message]
pub struct FsEvent {
    pub key_actions: Vec<KeyAction>,
    pub event: Event,
}
