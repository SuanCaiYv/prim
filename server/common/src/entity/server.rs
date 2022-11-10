use std::fmt::{Display, Formatter};
use tracing::error;
use crate::entity::{ServerInfo, ServerLoad, ServerStatus, ServerType};

impl Display for ServerStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerStatus::Online => write!(f, "Online"),
            ServerStatus::Normal => write!(f, "Normal"),
            ServerStatus::Overload => write!(f, "Overload"),
            ServerStatus::Crash => write!(f, "Crash"),
            ServerStatus::Offline => write!(f, "Offline"),
            _ => write!(f, "NA"),
        }
    }
}

impl Display for ServerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerType::ReplayCluster => write!(f, "ReplayCluster"),
            ServerType::BalancerCluster => write!(f, "BalancerCluster"),
            ServerType::BalancerNode => write!(f, "BalancerNode"),
            ServerType::MessageCluster => write!(f, "MessageCluster"),
            _ => write!(f, "NA"),
        }
    }
}

impl Default for ServerLoad {
    fn default() -> Self {
        ServerLoad {
            cpu: (0, 0.0),
            mem: (0, 0.0),
            net: (0, 0.0),
            disk: (0, 0.0),
            thread_num: 0,
            process_num: 0,
            physical_mem: 0.0,
            virtual_mem: 0.0,
            swap_disk: 0.0,
            disk_write: 0,
            disk_read: 0,
            net_write: 0,
            net_read: 0,
        }
    }
}

impl Display for ServerLoad {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cpu: {:?}, mem: {:?}, net: {:?}, disk: {:?}, thread_num: {}, process_num: {}, physical_mem: {}, virtual_mem: {}, swap_disk: {}, disk_write: {}, disk_read: {}, net_write: {}, net_read: {}",
               self.cpu, self.mem, self.net, self.disk, self.thread_num, self.process_num, self.physical_mem, self.virtual_mem, self.swap_disk, self.disk_write, self.disk_read, self.net_write, self.net_read)
    }
}

impl Default for ServerInfo {
    fn default() -> Self {
        ServerInfo {
            id: 0,
            address: "[::]:12345".parse().unwrap(),
            connection_id: 0,
            status: ServerStatus::NA,
            typ: ServerType::NA,
            load: None,
        }
    }
}

impl From<&[u8]> for ServerInfo {
    fn from(value: &[u8]) -> Self {
        let res: serde_json::Result<ServerInfo> = serde_json::from_slice(value);
        match res {
            Ok(v) => v,
            Err(e) => {
                error!("failed to deserialize ServerInfo from bytes: {}", e);
                ServerInfo::default()
            }
        }
    }
}

impl Display for ServerInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ServerInfo {{ id: {}, address: {}, connection_id: {}, status: {:?}, typ: {:?}, load: {:?} }}",
               self.id, self.address, self.connection_id, self.status, self.typ, self.load)
    }
}

impl ServerInfo {
    pub fn to_bytes(&self) -> Vec<u8> {
        let result = serde_json::to_vec(self);
        match result {
            Ok(v) => v,
            Err(e) => {
                error!("failed to serialize ServerInfo to bytes: {}", e);
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::ServerInfo;

    #[test]
    fn test() {
        let server_info = ServerInfo::default();
        let bytes = server_info.to_bytes();
        let server_info2 = ServerInfo::from(&bytes[..]);
        println!("{}", server_info2);
        assert_eq!(server_info, server_info2);
    }
}