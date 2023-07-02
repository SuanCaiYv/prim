use std::{
    cell::UnsafeCell,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    pin::Pin,
};

use ahash::AHashMap;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use futures::future::BoxFuture;
use lazy_static::lazy_static;
use lib::{
    entity::ReqwestMsg,
    net::{InnerStates},
    Result,
};
use local_sync::oneshot;
use lib_net_monoio::net::ReqwestHandler;
use tokio::sync::Mutex;
use tracing::info;

use crate::{util::{as_bytes, from_bytes}, config::CONFIG, service::SeqnumMap};

pub(self) const MAX_FILE_SIZE: u64 = 1024;

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
    inner: UnsafeCell<Option<oneshot::Receiver<()>>>,
}

pub(crate) struct SeqNum {
    /// to compatible with lib_net_tokio, we choose to use UnsafeCell instead of Rc,
    /// cause the Trait ReqwestHandler requires Send + Sync bound.
    /// in fact, there is only one thread using this file for operation, so data conflict
    /// will not happen.
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
            .await.unwrap();
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
        // if unsafe {&mut *self.file_entry.inner.get()}.1 > MAX_FILE_SIZE && unsafe {& *self.file_rx.inner.get()}.is_none() {
        //     let (tx, rx) = oneshot::channel();
        //     *unsafe { &mut *self.file_rx.inner.get() } = Some(rx);
        //     let new_file_path = format!(
        //         "{}/seqnum-{}",
        //         CONFIG.server.append_dir,
        //         ID.fetch_add(1, Ordering::SeqCst)
        //     );
        //     let new_file = monoio::fs::OpenOptions::new()
        //         .create(true)
        //         .custom_flags(0x0400)
        //         .append(true)
        //         .open(&new_file_path)
        //         .await?;
        //     let old_file_path = unsafe {& *self.file_path.inner.get()}.clone();
        //     *unsafe { &mut *self.file_path.inner.get() } = new_file_path.clone().into();
        //     *unsafe { &mut *self.file_entry.inner.get() } = (new_file, 0);
        //     monoio::spawn(async move {
        //         let mut buf = vec![0u8; 24];
        //         let mut res;
        //         let old_file = monoio::fs::OpenOptions::new()
        //             .read(true)
        //             .open(&old_file_path)
        //             .await
        //             .unwrap();
        //         let new_file = monoio::fs::OpenOptions::new()
        //             .create(true)
        //             .custom_flags(0x0400)
        //             .append(true)
        //             .open(&new_file_path)
        //             .await
        //             .unwrap();
        //         let mut map = AHashMap::new();
        //         loop {
        //             (res, buf) = old_file.read_exact_at(buf, 0).await;
        //             if res.is_err() {
        //                 break;
        //             }
        //             let (key, seq_num) = from_bytes(&buf[..]);
        //             map.entry(key)
        //                 .and_modify(|seqnum| {
        //                     if *seqnum < seq_num {
        //                         *seqnum = seq_num;
        //                     }
        //                 })
        //                 .or_insert(seq_num);
        //         }
        //         for (key, seqnum) in map {
        //             as_bytes(key, seqnum, &mut buf[..]);
        //             (res, buf) = new_file.write_all_at(buf, 0).await;
        //             if res.is_err() {
        //                 break;
        //             }
        //         }
        //         tx.send(()).unwrap();
        //         std::fs::remove_file(old_file_path).unwrap();
        //     });
        // }
        // if let Some(rx) = unsafe { &mut *self.file_rx.inner.get() }.as_mut() {
        //     if rx.try_recv().is_ok() {
        //         unsafe { &mut *self.file_rx.inner.get() }.take();
        //     }
        // }
        let mut buf = vec![0u8; 24];
        as_bytes(key, seqnum, &mut buf[..]);
        let (res, _buf) = unsafe { &mut *self.file_entry.inner.get() }.0.write_all_at(buf, 0).await;
        res?;
        unsafe {&mut *self.file_entry.inner.get()}.1 += 24;
        Ok(())
    }
}

#[async_trait(?Send)]
impl ReqwestHandler for SeqNum {
    async fn run(&self, msg: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let key = BigEndian::read_u128(msg.payload());
        let generic_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap();
        let seqnum_op = match generic_map.get_parameter::<SeqnumMap>()?.get(&key) {
            Some(seqnum) => (*seqnum).clone(),
            None => {
                let seqnum = Arc::new(AtomicU64::new(0));
                generic_map
                    .get_parameter::<SeqnumMap>()?
                    .insert(key, seqnum.clone());
                seqnum
            }
        };
        let seqnum = seqnum_op.fetch_add(1, Ordering::AcqRel);
        let t = std::time::Instant::now();
        if CONFIG.server.exactly_mode {
            self.save(key, seqnum).await?;
        } else {
            if seqnum & 0x7F == 0 {
                self.save(key, seqnum).await?;
            }
        };
        let mut buf = [0u8; 8];
        BigEndian::write_u64(&mut buf, seqnum);
        info!("cost: {:?}", t.elapsed());
        Ok(ReqwestMsg::with_resource_id_payload(
            msg.resource_id(),
            &buf,
        ))
    }
}
