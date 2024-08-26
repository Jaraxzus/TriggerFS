use elfo::prelude::*;

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
// #[message(part)]
// pub enum GroupFilter {
//     All,
//     ById(GroupId),
// }
//
// // Responses don't have to implement `Message`.
// #[message(part)]
// pub struct Summary {
//     pub group: GroupId,
//     pub sum: u32,
// }

// Wrappers can be marked as `transparent`, that adds `serde(transparent)`
// and implements `Debug` without printing the wrapper's name.
// #[message(part, transparent)]
// #[derive(Copy, PartialEq, Eq, Hash, derive_more::, derive_more::Display)]
// pub struct GroupId(u32);
