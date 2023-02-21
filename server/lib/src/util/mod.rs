use std::{
    net::{IpAddr, SocketAddr},
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
