use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use ahash::AHashMap;
use byteorder::{BigEndian, ByteOrder};
use lazy_static::lazy_static;
use thread_local::ThreadLocal;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};

use lib::Result;
use tracing::{info, log::warn, error};

pub(self) const MAX_FILE_SIZE: u64 = 96;

lazy_static! {
    static ref ID: AtomicU64 = AtomicU64::new(0);
}

pub(crate) async fn persistence_sequence_number_threshold(
    file_tls: &Arc<
        ThreadLocal<
            RefCell<
                Option<(
                    tokio::fs::File,
                    PathBuf,
                    Option<tokio::sync::oneshot::Receiver<PathBuf>>,
                )>,
            >,
        >,
    >,
    user_id: u64,
    peer_id: u64,
    seq_num: u64,
) -> Result<()> {
    let mut buf = [0u8; 24];
    let (user_id1, user_id2) = if user_id < peer_id {
        (user_id, peer_id)
    } else {
        (peer_id, user_id)
    };
    as_bytes(user_id1, user_id2, seq_num, &mut buf[..]);
    let file_option = &mut *match file_tls.get() {
        Some(file) => file.borrow_mut(),
        None => {
            let file_path_str = format!("seqnum-{}.out", ID.fetch_add(1, Ordering::AcqRel));
            let file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&file_path_str)
                .await
                .unwrap();
            let value = RefCell::new(Some((file, PathBuf::from(file_path_str), None)));
            file_tls.get_or(|| value).borrow_mut()
        }
    };
    let file = file_option.as_ref().unwrap();
    if file.0.metadata().await?.len() > MAX_FILE_SIZE && file.2.is_none() {
        info!("file size exceeds max file size, creating new file");
        let temp_seqnum_file_path_str = format!(
            "seqnum-temp-{}.out",
            get_file_id(&file.1.file_name().unwrap().to_str().unwrap())
        );
        let temp_seqnum_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .custom_flags(0x4000)
            .open(&temp_seqnum_file_path_str)
            .await
            .unwrap();
        let old_seqnum_file = file_option.take().unwrap();
        let (tx, rx) = oneshot::channel();
        file_option.replace((
            temp_seqnum_file,
            PathBuf::from(temp_seqnum_file_path_str),
            Some(rx),
        ));
        tokio::spawn(async move {
            let mut old_seqnum_file0 = tokio::fs::OpenOptions::new()
                .read(true)
                .open(&old_seqnum_file.1)
                .await?;
            let mut map = AHashMap::new();
            let mut archive_buf = [0u8; 24];
            while let Ok(_) = old_seqnum_file0.read_exact(&mut archive_buf[..]).await {
                let (user_id1, user_id2, seq_num) = from_bytes(&archive_buf[..]);
                map.entry((user_id1, user_id2))
                    .and_modify(|v| *v = seq_num)
                    .or_insert(seq_num);
            }
            let archive_seqnum_file_path_str = format!(
                "seqnum-archive-{}.out",
                get_file_id(&old_seqnum_file.1.file_name().unwrap().to_str().unwrap())
            );
            let mut archive_seqnum_file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&archive_seqnum_file_path_str)
                .await?;
            for ((user_id1, user_id2), seq_num) in map {
                as_bytes(user_id1, user_id2, seq_num, &mut archive_buf[..]);
                archive_seqnum_file.write_all(&archive_buf[..]).await?;
            }
            tokio::fs::remove_file(&old_seqnum_file.1).await?;
            warn!("delete {}", old_seqnum_file.1.as_os_str().to_str().unwrap());
            _ = tx.send(PathBuf::from(archive_seqnum_file_path_str));
            Result::<()>::Ok(())
        });
    }
    let mut file = file_option.as_mut().unwrap();
    if let Some(signal) = &mut file.2 {
        if let Ok(archive) = signal.try_recv() {
            let new_seqnum_file_path_str =
                format!("seqnum-{}.out", ID.fetch_add(1, Ordering::AcqRel));
            let mut new_seqnum_file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&new_seqnum_file_path_str)
                .await?;
            let mut archive_seqnum_file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(&archive)
                .await?;
            let mut temp_seqnum_file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(&file.1)
                .await?;
            let mut copy_buf = [0u8; 24 * 1024];
            info!("copying seqnum file: {}", archive_seqnum_file.metadata().await?.len());
            let mut index = 0;
            loop {
                match archive_seqnum_file.read(&mut copy_buf[index..]).await {
                    Ok(size) => {
                        if index == 0 {
                            break;
                        }
                        new_seqnum_file.write_all(&copy_buf[index..]).await?;
                        index += size;
                        if index == copy_buf.len() {
                            index = 0;
                        }
                    }
                    Err(e) => {
                        error!("read archive seqnum file error: {}", e);
                        break;
                    }
                }
            }
            info!("copying seqnum file: {}", temp_seqnum_file.metadata().await?.len());
            loop {
                match temp_seqnum_file.read(&mut copy_buf[index..]).await {
                    Ok(size) => {
                        if index == 0 {
                            break;
                        }
                        new_seqnum_file.write_all(&copy_buf[index..]).await?;
                        index += size;
                        if index == copy_buf.len() {
                            index = 0;
                        }
                    }
                    Err(e) => {
                        error!("read temp seqnum file error: {}", e);
                        break;
                    }
                }
            }
            tokio::fs::remove_file(&archive).await?;
            warn!("delete {}", archive.as_os_str().to_str().unwrap());
            tokio::fs::remove_file(&file.1).await?;
            warn!("delete {}", file.1.as_os_str().to_str().unwrap());
            file_option.replace((
                new_seqnum_file,
                PathBuf::from(new_seqnum_file_path_str),
                None,
            ));
            file = file_option.as_mut().unwrap();
        }
    }
    file.0.write_all(&buf[..]).await?;
    Ok(())
}

#[inline]
fn get_file_id(file_name: &str) -> u64 {
    file_name
        .split('.')
        .nth(0)
        .unwrap()
        .split('-')
        .last()
        .unwrap()
        .parse::<u64>()
        .unwrap()
}

fn as_bytes(user_id1: u64, user_id2: u64, seq_num: u64, buf: &mut [u8]) {
    BigEndian::write_u64(&mut buf[0..8], user_id1);
    BigEndian::write_u64(&mut buf[8..16], user_id2);
    BigEndian::write_u64(&mut buf[16..24], seq_num);
    // buf.copy_from_slice(format!("{:07}-{:07}:{:07}\n", user_id1, user_id2, seq_num).as_bytes());
}

fn from_bytes(buf: &[u8]) -> (u64, u64, u64) {
    (
        BigEndian::read_u64(&buf[0..8]),
        BigEndian::read_u64(&buf[8..16]),
        BigEndian::read_u64(&buf[16..24]),
    )
    // (
    //     String::from_utf8_lossy(&buf[0..7])
    //         .to_string()
    //         .parse::<u64>()
    //         .unwrap(),
    //     String::from_utf8_lossy(&buf[8..15])
    //         .to_string()
    //         .parse::<u64>()
    //         .unwrap(),
    //     String::from_utf8_lossy(&buf[16..23])
    //         .to_string()
    //         .parse::<u64>()
    //         .unwrap(),
    // )
}
