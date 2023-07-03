pub mod map;

use std::{
    time::{SystemTime, UNIX_EPOCH},
};

#[allow(unused)]
#[inline]
pub fn timestamp() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let millis = since_the_epoch.as_millis() as u64;
    millis
}

#[allow(unused)]
#[inline]
pub fn who_we_are(id1: u64, id2: u64) -> String {
    if id1 < id2 {
        format!("{}-{}", id1, id2)
    } else {
        format!("{}-{}", id2, id1)
    }
}

#[allow(unused)]
#[inline]
pub fn salt(length: usize) -> String {
    let length = if length > 32 { 32 } else { length };
    let string = uuid::Uuid::new_v4().to_string().replace("-", "M");
    String::from_utf8_lossy(&string.as_bytes()[0..length]).to_string().to_uppercase()
}

#[allow(unused)]
#[inline]
pub fn should_connect_to_peer(self_id: u32, peer_id: u32, new_peer: bool) -> bool {
    let peer_odd = peer_id & 1 == 1;
    let me_odd = self_id & 1 == 1;
    if peer_odd && me_odd {
        new_peer
    } else if peer_odd && !me_odd {
        !new_peer
    } else if !peer_odd && me_odd {
        !new_peer
    } else {
        new_peer
    }
}
