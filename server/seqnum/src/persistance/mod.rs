use std::{
    cell::RefCell,
    sync::{atomic::AtomicU16, Arc},
};

use byteorder::{BigEndian, ByteOrder};
use lazy_static::lazy_static;
use thread_local::ThreadLocal;

use lib::Result;

pub(self) const MAX_FILE_SIZE: u64 = 1024 * 1024 * 48;

lazy_static! {
    static ref ID: AtomicU16 = AtomicU16::new(0);
}

#[cfg(not(feature = "tokio_append"))]
pub(crate) async fn new_seq_num(
    file_thread_local: &Arc<ThreadLocal<RefCell<Option<std::fs::File>>>>,
    signal_thread_local: &Arc<ThreadLocal<RefCell<tokio::sync::oneshot::Receiver<std::fs::File>>>>,
    user_id: u64,
    peer_id: u64,
    seq_num: u64,
) -> Result<()> {
    use std::{io::Write, os::unix::prelude::OpenOptionsExt, sync::atomic::Ordering};

    let (user_id1, user_id2) = if user_id < peer_id {
        (user_id, peer_id)
    } else {
        (peer_id, user_id)
    };
    let mut buf = [0u8; 24];
    BigEndian::write_u64(&mut buf[0..8], user_id1);
    BigEndian::write_u64(&mut buf[8..16], user_id2);
    BigEndian::write_u64(&mut buf[16..24], seq_num);
    let file = file_thread_local.get_or(|| {
        RefCell::new(Some(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                // todo value set on macos, windows
                .custom_flags(0x4000)
                // todo file location set
                .open(&format!("seqnum-{}.out", ID.fetch_add(1, Ordering::AcqRel)))
                .unwrap(),
        ))
    });
    let file_option = &mut *file.borrow_mut();
    let mut file = file_option.as_mut().unwrap();
    let signal = signal_thread_local.get();
    if let Some(signal) = signal {
        let signal = &mut *signal.borrow_mut();
        if let Ok(new_file) = signal.try_recv() {
            let seqnum_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&format!(
                    "seqnum-{}.out",
                    ID.fetch_add(1, Ordering::AcqRel)
                ))?;
            let temp_file = file_option.take().unwrap();
            std::fs::remove_file()
        }
    } else {
        if file.metadata()?.len() > MAX_FILE_SIZE {
            let temp_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&format!(
                    "seqnum-temp-{}.out",
                    ID.fetch_add(1, Ordering::AcqRel)
                ))?;
            let old_file = file_option.take().unwrap();
            file_option.replace(temp_file);
            let (send_signal, recv_signal) = tokio::sync::oneshot::channel();
            signal_thread_local.get_or(|| RefCell::new(recv_signal));
            tokio::spawn(async move {});
            file = file_option.as_mut().unwrap();
        }
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
    }
    Ok(())
}

pub(crate) fn persistance_seq_num_start() -> Result<()> {
    Ok(())
}
