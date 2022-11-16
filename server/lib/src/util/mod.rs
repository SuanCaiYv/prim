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

#[allow(unused)]
#[inline]
/// this function will return local ip address, and version 6 is preferred.
pub fn my_ip() -> String {
    let list = local_ip_address::list_afinet_retinas().unwrap();
    if is_ipv6_enabled() {
        let ip = list
            .iter()
            .filter(|(name, addr)| {
                if name == "en0" {
                    if let IpAddr::V6(_) = addr {
                        return true;
                    }
                }
                false
            })
            .map(|x| x.1)
            .collect::<Vec<IpAddr>>();
        ip[1].to_string()
    } else {
        let ip = list
            .iter()
            .filter(|(name, addr)| {
                if name == "en0" {
                    if let IpAddr::V4(_) = addr {
                        return true;
                    }
                }
                false
            })
            .map(|x| x.1)
            .collect::<Vec<IpAddr>>();
        ip[0].to_string()
    }
}

#[allow(unused)]
#[inline]
/// this function may has some bugs, but in my test, it works well.
pub fn is_ipv6_enabled() -> bool {
    let list = local_ip_address::list_afinet_retinas().unwrap();
    let count = list
        .iter()
        .filter(|(name, addr)| {
            if name == "en0" {
                if let IpAddr::V6(_) = addr {
                    return true;
                }
            }
            false
        })
        .count();
    count > 1
}

#[allow(unused)]
#[inline]
pub fn default_bind_ip() -> SocketAddr {
    if is_ipv6_enabled() {
        "[::1]:0".parse().unwrap()
    } else {
        "127.0.0.1:0".parse().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::util::my_ip;

    #[test]
    fn test() {
        println!("my ip is {}", my_ip());
    }
}
