use std::net::SocketAddr;

pub mod msg;
pub mod server;

pub const HEAD_LEN: usize = 32;
pub const EXTENSION_THRESHOLD: usize = 1 << 6 - 1;
pub const PAYLOAD_THRESHOLD: usize = 1 << 14 - 1;

#[derive(
serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Hash,
)]
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
    Noop,
    Replay,
    NodeRegister,
    NodeUnregister,
    UserNodeMapChange,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Head {
    /// constituted of 18 bit version and 46 bit user id
    pub(self) version_with_sender: u64,
    /// constituted of 18 bit node id and 46 bit user id
    pub(self) node_id_with_receiver: u64,
    /// constituted of 12 bit type, 6 bit extension length and 46 bit timestamp
    pub(self) type_with_extension_length_with_timestamp: u64,
    /// constituted of 14 bit payload length and 50 bit seq num.
    pub(self) payload_length_with_seq_num: u64,
}

pub(crate) struct InnerHead {
    pub(self) version: u32,
    pub(self) sender: u64,
    pub(self) node_id: u32,
    pub(self) receiver: u64,
    pub(self) typ: Type,
    pub(self) extension_length: u8,
    pub(self) timestamp: u64,
    pub(self) payload_length: u16,
    pub(self) seq_num: u64,
}

/// a message's layout may look like:
/// ```
/// use lib::entity::Head;
/// struct Msg {
///     head: Head,
///     payload: Vec<u8>,
///     extension: Vec<u8>,
/// }
/// ```
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Msg(pub Vec<u8>);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    NA,
    Online,
    Normal,
    Overload,
    Crash,
    Offline,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    NA,
    SchedulerCluster,
    SchedulerClient,
    MessageCluster,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct ServerLoad {
    pub cpu: (u32, f32),
    pub mem: (u32, f32),
    pub net: (u32, f32),
    pub disk: (u32, f32),
    pub thread_num: u32,
    pub process_num: u32,
    pub physical_mem: f32,
    pub virtual_mem: f32,
    pub swap_disk: f32,
    pub disk_write: u32,
    pub disk_read: u32,
    pub net_write: u32,
    pub net_read: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct ServerInfo {
    pub id: u32,
    pub address: SocketAddr,
    /// from the view of connected endpoint
    pub connection_id: u64,
    pub status: ServerStatus,
    pub typ: ServerType,
    pub load: Option<ServerLoad>,
}
