use crate::cache::{get_redis_ops, NODE_ID_KEY};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod time_queue;

pub(crate) static mut MY_ID: u32 = 0;

#[inline]
pub(crate) fn my_id() -> u32 {
    unsafe { MY_ID }
}

pub(crate) async fn load_my_id(my_id_preload: u32) -> common::Result<()> {
    if my_id_preload != 0 {
        unsafe { MY_ID = my_id_preload };
        return Ok(());
    }
    let path = PathBuf::from("./my_id");
    let path = path.as_path();
    let file = tokio::fs::File::open(path).await;
    let my_id;
    if let Ok(file) = file {
        let mut reader = tokio::io::BufReader::new(file);
        let mut s = String::new();
        reader.read_to_string(&mut s).await?;
        my_id = s.parse::<u32>()?;
    } else {
        let mut file = tokio::fs::File::create(path).await?;
        my_id = get_redis_ops()
            .await
            .atomic_increment(NODE_ID_KEY.to_string())
            .await
            .unwrap() as u32;
        let s = my_id.to_string();
        file.write_all(s.as_bytes()).await?;
        file.flush().await?;
    }
    unsafe { MY_ID = my_id }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}

/*
lo0 127.0.0.1
lo0 ::1
lo0 fe80::1
anpi1 fe80::e45e:80ff:fef2:9582
anpi0 fe80::e45e:80ff:fef2:9581
anpi2 fe80::e45e:80ff:fef2:9583
en0 fe80::4f3:6670:9684:2597
en0 192.168.11.214
awdl0 fe80::dc5f:f2ff:fed2:c728
llw0 fe80::dc5f:f2ff:fed2:c728
utun0 fe80::3c29:ed6:172f:6079
utun1 fe80::b492:79a1:cab:f2c5
utun2 fe80::ce81:b1c:bd2c:69e
utun3 fe80::1093:94f1:1828:a3d
utun4 fe80::505a:9c1c:9b1e:95c2
utun5 fe80::2f0d:4172:c624:2dc9
utun6 fe80::c8b9:9038:27d8:1601
fe80::4f3:6670:9684:2597

lo0 127.0.0.1
lo0 ::1
lo0 fe80::1
anpi1 fe80::e45e:80ff:fef2:9582
anpi0 fe80::e45e:80ff:fef2:9581
anpi2 fe80::e45e:80ff:fef2:9583
en0 fe80::4f3:6670:9684:2597
en0 172.20.10.6
en0 2408:840d:5b10:6c6:1420:22bd:8d79:bc22
en0 2408:840d:5b10:6c6:69da:bb8b:83ed:140c
awdl0 fe80::3c62:e0ff:fe1f:c156
llw0 fe80::3c62:e0ff:fe1f:c156
utun0 fe80::3c29:ed6:172f:6079
utun1 fe80::b492:79a1:cab:f2c5
utun2 fe80::ce81:b1c:bd2c:69e
utun3 fe80::1093:94f1:1828:a3d
utun4 fe80::505a:9c1c:9b1e:95c2
utun5 fe80::2f0d:4172:c624:2dc9
utun6 fe80::c8b9:9038:27d8:1601
fe80::4f3:6670:9684:2597
 */
