use std::time::{SystemTime, UNIX_EPOCH};

pub mod jwt;

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
pub fn exactly_time() -> (u64, u64) {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    (
        duration.as_secs(),
        (duration.as_nanos() % 1_000_000_000 as u128) as u64,
    )
}

#[allow(unused)]
#[inline]
pub fn nanos_time() -> u128 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    duration.as_nanos()
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
pub fn salt() -> String {
    let string = uuid::Uuid::new_v4().to_string().replace("-", "V");
    String::from_utf8_lossy(&string.as_bytes()[0..32]).to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        println!("{}", super::timestamp());
        let a = super::exactly_time();
        println!("{} {}", a.0 * 1000, a.1 / 1_000_000);
    }
}
