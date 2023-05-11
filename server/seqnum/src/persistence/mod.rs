use std::{
    cell::RefCell,
    sync::{Arc, atomic::AtomicU64}, io::Read,
};

use ahash::AHashMap;
use byteorder::{BigEndian, ByteOrder};
use lazy_static::lazy_static;
use thread_local::ThreadLocal;
use tracing::{debug, info, warn};

use lib::Result;

pub(self) const MAX_FILE_SIZE: u64 = 48;

lazy_static! {
    static ref ID: AtomicU64 = AtomicU64::new(0);
}

#[cfg(not(feature = "tokio_append"))]
pub(crate) fn persistence_new_seq_num(
    file_thread_local: &Arc<ThreadLocal<RefCell<Option<(std::fs::File, std::path::PathBuf)>>>>,
    signal_thread_local: &Arc<ThreadLocal<RefCell<tokio::sync::oneshot::Receiver<std::path::PathBuf>>>>,
    user_id: u64,
    peer_id: u64,
    seq_num: u64,
) -> Result<()> {
    use std::{
        io::Write, os::unix::prelude::OpenOptionsExt, path::PathBuf, sync::atomic::Ordering,
    };

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
        // todo file location set
        let path_str = format!("seqnum-{}.out", ID.fetch_add(1, Ordering::AcqRel));
        RefCell::new(Some((
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                // todo value set on macos, windows
                .custom_flags(0x4000)
                .open(&path_str)
                .unwrap(),
            PathBuf::from(path_str),
        )))
    });
    let file_option = &mut *file.borrow_mut();
    let mut file = file_option.as_mut().unwrap();
    let signal = signal_thread_local.get();
    if let Some(signal) = signal {
        let signal = &mut *signal.borrow_mut();
        if let Ok(mut new_file_path) = signal.try_recv() {
            let new_seqnum_file_path;
            let mut seqnum_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(new_file_path.as_path())?;
            let mut new_file = std::fs::OpenOptions::new()
                .read(true)
                .open(&new_file_path)?;
            let mut temp_file = file_option.take().unwrap();
            let mut copy_buf = [0u8; 24];
            while let Ok(_) = new_file.read_exact(&mut copy_buf[..]) {
                info!("write2 seqnum: {:?}", &copy_buf[..]);
                seqnum_file.write_all(&copy_buf[..])?;
            }
            while let Ok(_) = temp_file.0.read_exact(&mut copy_buf[..]) {
                info!("write2 seqnum: {:?}", &copy_buf[..]);
                seqnum_file.write_all(&copy_buf[..])?;
            }
            file_option.replace((seqnum_file, new_file_path.1));
            std::fs::remove_file(temp_file.1)?;
            file = file_option.as_mut().unwrap();
        }
    } else {
        if file.0.metadata()?.len() >= MAX_FILE_SIZE {
            let file_id = file
                .1
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .split('.')
                .nth(0)
                .unwrap()
                .split('-')
                .last()
                .unwrap()
                .parse::<u16>()
                .unwrap();
            let temp_file_path = format!("seqnum-temp-{}.out", file_id);
            let temp_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&temp_file_path)?;
            let mut old_file = file_option.take().unwrap();
            file_option.replace((temp_file, PathBuf::from(temp_file_path)));
            let (send_signal, recv_signal) = tokio::sync::oneshot::channel();
            signal_thread_local.get_or(|| RefCell::new(recv_signal));
            tokio::spawn(async move {
                let new_file_path = format!("seqnum-archive-{}.out", ID.fetch_add(1, Ordering::AcqRel));
                let mut new_file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .custom_flags(0x4000)
                    .open(&new_file_path)?;
                let mut old_seq_file = std::fs::OpenOptions::new().read(true).open(&old_file.1)?;
                let mut map = AHashMap::new();
                let mut archive_buf = [0u8; 24];
                while let Ok(_) = old_seq_file.read_exact(&mut archive_buf[..]) {
                    let user_id1 = BigEndian::read_u64(&archive_buf[0..8]);
                    let user_id2 = BigEndian::read_u64(&archive_buf[8..16]);
                    let seq_num = BigEndian::read_u64(&archive_buf[16..24]);
                    debug!("archive seqnum: {:?}", &archive_buf[..]);
                    map.entry((user_id1, user_id2))
                        .and_modify(|v| *v = seq_num)
                        .or_insert(seq_num);
                }
                debug!("archive map: {:?}", map);
                map.iter().for_each(|(k, v)| {
                    let mut buf = [0u8; 24];
                    BigEndian::write_u64(&mut buf[0..8], k.0);
                    BigEndian::write_u64(&mut buf[8..16], k.1);
                    BigEndian::write_u64(&mut buf[16..24], *v);
                    info!("write1 seqnum: {:?}", &buf[..]);
                    new_file.write_all(&buf[..]).unwrap();
                });
                std::fs::remove_file(&old_file.1)?;
                send_signal.send(PathBuf::from(new_file_path)).unwrap();
                Result::<()>::Ok(())
            });
            file = file_option.as_mut().unwrap();
        }
    }
    file.0.write_all(&buf[..])?;
    info!("write0 seqnum: {:?}", &buf[..]);
    Ok(())
}

pub(crate) async fn persistence_sequence_number_threshold(file_tls: ThreadLocal<RefCell<Option<(std::path::PathBuf, tokio::fs::File)>>>) -> Result<()> {
    Ok(())
}
