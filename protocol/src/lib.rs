use std::{
    fmt::{Debug, Display},
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use elfo::prelude::*;
use fs::actions::Action;
use notify::Event;

// It's just a regular message.
// `message` derives
// * `Debug` for logging in dev env
// * `Serialize` and `Deserialize` for dumping and comminication between nodes
// * `Message` and `Request` to restrict contracts
// #[message]
// pub struct AddNum {
//     pub group: GroupId,
//     pub num: u32,
// }
//
// // Messages with specified `ret` are requests.
// #[message(ret = Summary)]
// pub struct Summarize {
//     pub group_filter: GroupFilter,
// }
//
// // Parts of messages can be marked with `message(part)`
// // to derive `Debug`, `Clone`, `Serialize` and `Deserialize`.

//
// // Responses don't have to implement `Message`.
// #[message(part)]
// pub struct Summary {
//     pub group: GroupId,
//     pub sum: u32,
// }
//
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
