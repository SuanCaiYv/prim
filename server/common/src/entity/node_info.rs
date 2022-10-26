use byteorder::BigEndian;
use crate::entity::NodeInfo;

impl From<&[u8]> for NodeInfo {
    fn from(buf: &[u8]) -> Self {
        let res: serde_json::Result<NodeInfo> = serde_json::from_slice(buf);
        if res.is_err() {
            Self {
                node_id: 0,
                address: "[::1]:8190".parse().expect("parse address failed"),
                connection_id: 0,
                status: 0,
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