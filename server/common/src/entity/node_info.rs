use crate::entity::{NodeInfo, NodeStatus};

impl From<u8> for NodeStatus {
    fn from(v: u8) -> Self {
        match v {
            1 => NodeStatus::DirectRegister,
            2 => NodeStatus::ClusterRegister,
            3 => NodeStatus::DirectUnregister,
            4 => NodeStatus::ClusterUnregister,
            _ => panic!("invalid node status"),
        }
    }
}

impl From<NodeStatus> for u8 {
    fn from(v: NodeStatus) -> Self {
        match v {
            NodeStatus::DirectRegister => 1,
            NodeStatus::ClusterRegister => 2,
            NodeStatus::DirectUnregister => 3,
            NodeStatus::ClusterUnregister => 4,
        }
    }
}

impl From<&NodeStatus> for u8 {
    fn from(v: &NodeStatus) -> Self {
        match *v {
            NodeStatus::DirectRegister => 1,
            NodeStatus::ClusterRegister => 2,
            NodeStatus::DirectUnregister => 3,
            NodeStatus::ClusterUnregister => 4,
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
                status: NodeStatus::DirectRegister,
            }
        } else {
            res.unwrap()
        }
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