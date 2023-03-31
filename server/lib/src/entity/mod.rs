use std::net::SocketAddr;

use num_derive::FromPrimitive;

pub mod msg;
pub mod server;

pub const HEAD_LEN: usize = 32;
pub const EXTENSION_THRESHOLD: usize = 1 << 6 - 1;
pub const PAYLOAD_THRESHOLD: usize = 1 << 14 - 1;
/// user_id lager than(also equal) this value is considered as a group
pub const GROUP_ID_THRESHOLD: u64 = 1 << 36;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    sqlx::Type,
    Hash,
    FromPrimitive,
)]
pub enum Type {
    NA = 0,
    /// this type can only be used for acknowledging certain msg.
    /// it's so special that we put it on the top of the enum.
    Ack = 1,

    /// the below types are used for user's communication.
    ///
    /// pure message part
    Text = 32,
    Meme = 33,
    File = 34,
    Image = 35,
    Video = 36,
    Audio = 37,
    /// control message part
    Edit = 64,
    Withdraw = 65,

    /// the below types are used for user and server's communication.
    ///
    /// logic part
    Auth = 96,
    Ping = 97,
    Pong = 98,
    Echo = 99,
    Error = 100,
    BeOffline = 101,
    InternalError = 102,
    /// business part
    /// some types may derived by user but send between server, those types are also viewed as business type.
    SystemMessage = 128,
    AddFriend = 129,
    RemoveFriend = 130,
    JoinGroup = 131,
    LeaveGroup = 132,
    RemoteInvoke = 133,
    SetRelationship = 134,

    /// the below types are used for server's communication.
    ///
    /// internal part
    /// this part should never be visible to the user end.
    Noop = 160,
    InterruptSignal = 161,
    UserNodeMapChange = 162,
    MessageNodeRegister = 163,
    MessageNodeUnregister = 164,
    RecorderNodeRegister = 165,
    RecorderNodeUnregister = 166,
    SchedulerNodeRegister = 167,
    SchedulerNodeUnregister = 168,
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

/// a tiny message's layout may look like:
/// ```
/// struct TinyMsg {
///    length: u16,
///    payload: Vec<u8>,
/// }
/// ```
pub struct TinyMsg(pub Vec<u8>);

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
    RecorderCluster,
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
    pub cluster_address: Option<SocketAddr>,
    pub service_address: SocketAddr,
    /// from the view of connected endpoint
    pub connection_id: u64,
    pub status: ServerStatus,
    pub typ: ServerType,
    pub load: Option<ServerLoad>,
}
