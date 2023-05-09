use std::io::Write;

use byteorder::{BigEndian, ByteOrder};
use lib::Result;
#[allow(unused)]
use tokio::io::AsyncWriteExt;

#[cfg(not(feature = "tokio_append"))]
static mut APPEND_FILE: Option<std::fs::File> = None;

#[cfg(feature = "tokio_append")]
static mut APPEND_FILE: Option<tokio::fs::File> = None;

pub(crate) async fn new_seq_num(user_id: u64, peer_id: u64, seq_num: u64) -> Result<()> {
    let (user_id1, user_id2) = if user_id < peer_id {
        (user_id, peer_id)
    } else {
        (peer_id, user_id)
    };
    let mut buf = [0u8; 24];
    #[cfg(not(feature = "tokio_append"))]
    unsafe {
        match APPEND_FILE {
            Some(ref mut file) => {
                BigEndian::write_u64(&mut buf[0..8], user_id1);
                BigEndian::write_u64(&mut buf[8..16], user_id2);
                BigEndian::write_u64(&mut buf[16..24], seq_num);
                let mut index = 0;
                loop {
                    match file.write(&buf[index..]) {
                        Ok(size) => {
                            if index + size == buf.len() {
                                break;
                            } else {
                                index += size;
                            }
                        }
                        Err(e) => {
                            // just panic
                            panic!("write error: {}", e);
                        }
                    }
                }
                Ok(())
            }
            None => {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    // todo file location set
                    .open("seqnum.out")?;
                BigEndian::write_u64(&mut buf[0..8], user_id1);
                BigEndian::write_u64(&mut buf[8..16], user_id2);
                BigEndian::write_u64(&mut buf[16..24], seq_num);
                let mut index = 0;
                loop {
                    match file.write(&buf[index..]) {
                        Ok(size) => {
                            if index + size == buf.len() {
                                break;
                            } else {
                                index += size;
                            }
                        }
                        Err(e) => {
                            // just panic
                            panic!("write error: {}", e);
                        }
                    }
                }
                APPEND_FILE = Some(file);
                Ok(())
            }
        }
    }
    #[cfg(feature = "tokio_append")]
    unsafe {
        match APPEND_FILE {
            Some(ref mut file) => {
                BigEndian::write_u64(&mut buf[0..8], user_id1);
                BigEndian::write_u64(&mut buf[8..16], user_id2);
                BigEndian::write_u64(&mut buf[16..24], seq_num);
                match file.write_all(&buf[..]).await {
                    Ok(_) => {}
                    Err(e) => {
                        // just panic
                        panic!("write error: {}", e);
                    }
                }
                Ok(())
            }
            None => {
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    // todo file location set
                    .open("seqnum")
                    .await?;
                BigEndian::write_u64(&mut buf[0..8], user_id1);
                BigEndian::write_u64(&mut buf[8..16], user_id2);
                BigEndian::write_u64(&mut buf[16..24], seq_num);
                match file.write_all(&buf[..]).await {
                    Ok(_) => {}
                    Err(e) => {
                        // just panic
                        panic!("write error: {}", e);
                    }
                }
                APPEND_FILE = Some(file);
                Ok(())
            }
        }
    }
}

pub(crate) fn persistance_seq_num_start() -> Result<()> {
    Ok(())
}
