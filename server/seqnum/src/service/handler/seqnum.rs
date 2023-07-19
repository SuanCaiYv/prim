use std::{
    cell::UnsafeCell,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use ahash::AHashMap;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use dashmap::mapref::entry::Entry;
use lazy_static::lazy_static;
use lib::{entity::ReqwestMsg, net::InnerStates, Result};
use lib_net_monoio::net::ReqwestHandler;
use local_sync::oneshot;

use crate::{
    config::CONFIG,
    service::SeqnumMap,
    util::{as_bytes, from_bytes},
};

pub(self) const MAX_FILE_SIZE: u64 = 24 << 28;
pub(crate) const SAVE_THRESHOLD: u64 = 0x4000;

lazy_static! {
    static ref ID: AtomicU64 = AtomicU64::new(0);
}

pub(self) struct FilePath {
    inner: UnsafeCell<PathBuf>,
}

pub(self) struct FileEntry {
    inner: UnsafeCell<(monoio::fs::File, u64)>,
}

pub(self) struct FileRx {
    inner: UnsafeCell<Option<oneshot::Receiver<u64>>>,
}

pub(crate) struct SeqNum {
    file_rx: FileRx,
    file_path: FilePath,
    file_entry: FileEntry,
}

impl SeqNum {
    pub(crate) async fn new() -> Self {
        let file_path = format!(
            "{}/seqnum-{}",
            CONFIG.server.append_dir,
            ID.fetch_add(1, Ordering::SeqCst)
        );
        let file = monoio::fs::OpenOptions::new()
            .create(true)
            .custom_flags(0x0400)
            .append(true)
            .open(&file_path)
            .await
            .unwrap();
        let file_len = std::fs::metadata(&file_path).unwrap().len();
        Self {
            file_rx: FileRx {
                inner: UnsafeCell::new(None),
            },
            file_path: FilePath {
                inner: UnsafeCell::new(PathBuf::from(file_path)),
            },
            file_entry: FileEntry {
                inner: UnsafeCell::new((file, file_len)),
            },
        }
    }

    pub(self) async fn save(&self, key: u128, seqnum: u64) -> Result<()> {
        if unsafe { &mut *self.file_entry.inner.get() }.1 > MAX_FILE_SIZE
            && unsafe { &*self.file_rx.inner.get() }.is_none()
        {
            let (tx, rx) = oneshot::channel();
            *unsafe { &mut *self.file_rx.inner.get() } = Some(rx);
            let new_file_path = format!(
                "{}/seqnum-{}",
                CONFIG.server.append_dir,
                ID.fetch_add(1, Ordering::SeqCst)
            );
            let new_file = monoio::fs::OpenOptions::new()
                .create(true)
                .custom_flags(0x0400)
                .append(true)
                .open(&new_file_path)
                .await?;
            let new_file_len = std::fs::metadata(&new_file_path)?.len();
            let old_file_path = unsafe { &*self.file_path.inner.get() }.clone();
            *unsafe { &mut *self.file_path.inner.get() } = new_file_path.clone().into();
            *unsafe { &mut *self.file_entry.inner.get() } = (new_file, new_file_len);
            monoio::spawn(async move {
                let mut buf = vec![0u8; 24];
                let mut res;
                let old_file = monoio::fs::OpenOptions::new()
                    .read(true)
                    .open(&old_file_path)
                    .await
                    .unwrap();
                let new_file = monoio::fs::OpenOptions::new()
                    .create(true)
                    .custom_flags(0x0400)
                    .append(true)
                    .open(&new_file_path)
                    .await
                    .unwrap();
                let mut map = AHashMap::new();
                loop {
                    (res, buf) = old_file.read_exact_at(buf, 0).await;
                    if res.is_err() {
                        break;
                    }
                    let (key, seq_num) = from_bytes(&buf[..]);
                    map.entry(key)
                        .and_modify(|seqnum| {
                            if *seqnum < seq_num {
                                *seqnum = seq_num;
                            }
                        })
                        .or_insert(seq_num);
                }
                let mut len = 0;
                buf = vec![0u8; 24];
                for (key, seqnum) in map {
                    as_bytes(key, seqnum, &mut buf[..]);
                    (res, buf) = new_file.write_all_at(buf, 0).await;
                    len += 24;
                    if res.is_err() {
                        break;
                    }
                }
                tx.send(len).unwrap();
                std::fs::remove_file(old_file_path).unwrap();
            });
        }
        if let Some(rx) = unsafe { &mut *self.file_rx.inner.get() }.as_mut() {
            if let Ok(len) = rx.try_recv() {
                unsafe { &mut *self.file_entry.inner.get() }.1 += len;
                unsafe { &mut *self.file_rx.inner.get() }.take();
            }
        }
        let mut buf = vec![0u8; 24];
        as_bytes(key, seqnum, &mut buf[..]);
        let (res, _buf) = unsafe { &mut *self.file_entry.inner.get() }
            .0
            .write_all_at(buf, 0)
            .await;
        res?;
        unsafe { &mut *self.file_entry.inner.get() }.1 += 24;
        Ok(())
    }
}

#[async_trait(? Send)]
impl ReqwestHandler for SeqNum {
    async fn run(&self, msg: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let key = BigEndian::read_u128(msg.payload());
        let generic_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap();
        let seqnum;
        {
            seqnum = match generic_map
                .get_parameter::<SeqnumMap>()
                .unwrap()
                .0
                .entry(key)
            {
                Entry::Occupied(v) => v.get().fetch_add(1, Ordering::Acquire),
                Entry::Vacant(v) => {
                    let seqnum = AtomicU64::new(2);
                    v.insert(seqnum);
                    1
                }
            };
        }
        if CONFIG.server.exactly_mode {
            self.save(key, seqnum).await?;
        } else {
            // x & (2^n - 1) = x % 2^n
            if seqnum & (SAVE_THRESHOLD - 1) == 0 {
                self.save(key, seqnum).await?;
            }
        };
        let mut buf = [0u8; 8];
        BigEndian::write_u64(&mut buf, seqnum);
        Ok(ReqwestMsg::with_resource_id_payload(
            msg.resource_id(),
            &buf,
        ))
    }
}
