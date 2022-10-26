use std::net::SocketAddr;

pub mod msg;
pub mod node_info;

pub const HEAD_LEN: usize = 40;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, sqlx::Type)]
pub enum Type {
    NA,
    // message part
    Text,
    Meme,
    File,
    Image,
    Video,
    Audio,
    // logic part
    Ack,
    Auth,
    Ping,
    Echo,
    Error,
    Offline,
    UnderReview,
    InternalError,
    // business part
    SysNotification,
    FriendRelationship,
    // internal part
    Register,
    Unregister,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Head {
    // length od extension(in bytes)
    pub(crate) extension_length: u16,
    // length of payload(in bytes)
    pub(crate) payload_length: u16,
    // u16 size
    pub(crate) typ: Type,
    pub(crate) sender: u64,
    pub(crate) receiver: u64,
    pub(crate) timestamp: u64,
    pub(crate) seq_num: u64,
    // message version
    pub(crate) version: u16,
}

/// a message's layout may look like:
/// ```
/// use common::entity::Head;
/// struct Msg {
///     head: Head,
///     extension: Vec<u8>,
///     payload: Vec<u8>,
/// }
/// ```
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Msg(pub Vec<u8>);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NodeInfo {
    pub node_id: u64,
    pub address: SocketAddr,
    /// from the point of balancer
    pub connection_id: u64,
    pub status: i8,
}
