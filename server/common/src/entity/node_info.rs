use std::fmt::Display;
use crate::entity::{NodeInfo, NodeStatus};

impl From<u8> for NodeStatus {
    fn from(v: u8) -> Self {
        match v {
            1 => NodeStatus::Online,
            2 => NodeStatus::Normal,
            3 => NodeStatus::Overload,
            4 => NodeStatus::Crash,
            5 => NodeStatus::Offline,
            _ => panic!("invalid node status"),
        }
    }
}

impl From<NodeStatus> for u8 {
    fn from(v: NodeStatus) -> Self {
        match v {
            NodeStatus::Online => 1,
            NodeStatus::Normal => 2,
            NodeStatus::Overload => 3,
            NodeStatus::Crash => 4,
            NodeStatus::Offline => 5,
        }
    }
}

impl From<&NodeStatus> for u8 {
    fn from(v: &NodeStatus) -> Self {
        Self::from(*v)
    }
}

impl Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Online => write!(f, "online"),
            NodeStatus::Normal => write!(f, "normal"),
            NodeStatus::Overload => write!(f, "overload"),
            NodeStatus::Crash => write!(f, "crash"),
            NodeStatus::Offline => write!(f, "offline"),
        }
    }
}

impl From<&[u8]> for NodeInfo {
    fn from(buf: &[u8]) -> Self {
        let res: serde_json::Result<NodeInfo> = serde_json::from_slice(buf);
        if res.is_err() {
            Self {
                node_id: 0,
                address: "[::1]:8190".parse().expect("parse address failed"),
                connection_id: 0,
                status: NodeStatus::Offline,
            }
        } else {
            res.unwrap()
        }
    }
}

impl Display for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node_id: {}, address: {}, connection_id: {}, status: {}", self.node_id, self.address, self.connection_id, self.status)
    }
}

impl NodeInfo {
    pub fn to_bytes(&self) -> Vec<u8> {
        let res = serde_json::to_vec(self);
        if res.is_err() {
            vec![]
        } else {
            res.unwrap()
        }
    }
}