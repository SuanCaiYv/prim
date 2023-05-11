use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::{cell::RefCell, sync::atomic::AtomicU64};

use byteorder::{BigEndian, ByteOrder};
use lazy_static::lazy_static;
use thread_local::ThreadLocal;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use lib::Result;

pub(self) const MAX_FILE_SIZE: u64 = 48;

lazy_static! {
    static ref ID: AtomicU64 = AtomicU64::new(0);
}

pub(crate) async fn persistence_sequence_number_threshold(
    file_tls: ThreadLocal<
        RefCell<
            Option<(
                tokio::fs::File,
                PathBuf,
                Option<tokio::sync::oneshot::Receiver<Vec<u8>>>,
            )>,
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
    BigEndian::write_u64(&mut buf[0..8], user_id1);
    BigEndian::write_u64(&mut buf[8..16], user_id2);
    BigEndian::write_u64(&mut buf[16..24], seq_num);
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
                .await
                .unwrap();
            let mut index = 0;
            loop {
                if archive.len() - 24 * 1024 - index < 0 {
                    new_seqnum_file.write_all(&archive[index..]).await?;
                    break;
                } else {
                    new_seqnum_file
                        .write_all(&archive[index..index + 24 * 1024])
                        .await?;
                    index += 24 * 1024;
                }
            }
            let mut temp_seqnum_file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(&file.1)
                .await?;
            let mut copy_buf = [0u8; 24 * 1024];
            while let Ok(_) = temp_seqnum_file.read_exact(&mut copy_buf).await {
                new_seqnum_file.write_all(&copy_buf).await?;
            }
            tokio::fs::remove_file(&file.1).await?;
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
