use std::net::SocketAddr;

pub mod msg;
pub mod node_info;

pub const HEAD_LEN: usize = 48;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
pub enum Type {
    NA,
    /// message part
    Text,
    Meme,
    File,
    Image,
    Video,
    Audio,
    /// this one can only be used for acknowledging certain msg.
    Ack,
    /// logic part
    Auth,
    Ping,
    Echo,
    Error,
    BeOffline,
    InternalError,
    /// business part
    SystemMessage,
    AddFriend,
    RemoveFriend,
    JoinGroup,
    LeaveGroup,
    /// internal part
    NodeRegister,
    NodeUnregister,
    UserNodeMapChange,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Head {
    /// length od extension(in bytes)
    pub(self) extension_length: u16,
    /// length of payload(in bytes)
    pub(self) payload_length: u16,
    /// u16 size
    pub(self) typ: Type,
    pub(self) sender: u64,
    pub(self) receiver: u64,
    /// as cache of node_id
    pub(self) sender_node: u32,
    pub(self) receiver_node: u32,
    pub(self) timestamp: u64,
    pub(self) seq_num: u64,
    /// message version
    pub(self) version: u16,
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub enum NodeStatus {
    Online,
    Normal,
    Overload,
    Crash,
    Offline,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NodeInfo {
    pub node_id: u32,
    pub address: SocketAddr,
    /// from the point of balancer
    pub connection_id: u64,
    pub status: NodeStatus,
}
