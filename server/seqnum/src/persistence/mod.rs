use std::{
    cell::UnsafeCell,
    os::unix::prelude::OpenOptionsExt,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use ahash::AHashMap;
use byteorder::{BigEndian, ByteOrder};
use futures_util::FutureExt;
use lazy_static::lazy_static;
use thread_local::ThreadLocal;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};

use lib::Result;
use tracing::error;

use crate::{config::CONFIG, service::get_seqnum_map};

pub(self) const MAX_FILE_SIZE: u64 = 96;

lazy_static! {
    static ref ID: AtomicU64 = AtomicU64::new(0);
    static ref FILE: ThreadLocal<InnerStateMutable> = ThreadLocal::new();
}

pub(self) struct InnerState {
    file_path: PathBuf,
    completed: Option<oneshot::Receiver<()>>,
}

pub(self) struct InnerStateMutable {
    inner: UnsafeCell<InnerState>,
}

unsafe impl Send for InnerStateMutable {}

unsafe impl Sync for InnerStateMutable {}

pub(crate) async fn load() -> Result<()> {
    let mut map = AHashMap::new();
    let mut buf = [0u8; 24];
    let mut dir = tokio::fs::read_dir(".").await?;
    while let Some(entry) = dir.next_entry().await? {
        let file_name = entry.file_name();
        if let Some(file_name_str) = file_name.to_str() {
            if file_name_str.starts_with("seqnum-") {
                let mut file = tokio::fs::OpenOptions::new()
                    .read(true)
                    .open(&file_name)
                    .await?;
                while let Ok(_) = file.read_exact(&mut buf[..]).await {
                    let (key, seq_num) = from_bytes(&buf[..]);
                    map.entry(key)
                        .and_modify(|seqnum| {
                            if *seqnum < seq_num {
                                *seqnum = seq_num;
                            }
                        })
                        .or_insert(seq_num);
                }
            }
        }
    }
    let seqnum_map = get_seqnum_map();
    for (key, seqnum) in map {
        seqnum_map.insert(key, Arc::new(AtomicU64::new(seqnum)));
    }
    Ok(())
}

pub(crate) async fn save(key: u128, seqnum: u64) -> Result<()> {
    let mut buf = [0u8; 24];
    as_bytes(key, seqnum, &mut buf[..]);
    let file_state = FILE.get_or(|| InnerStateMutable {
        inner: UnsafeCell::new(InnerState {
            file_path: PathBuf::from(format!(
                "{}/seqnum-{}.out",
                CONFIG.server.append_dir,
                ID.fetch_add(1, Ordering::AcqRel)
            )),
            completed: None,
        }),
    });
    let completed = &mut unsafe { &mut *file_state.inner.get() }.completed;
    if let Some(completed) = completed.as_mut() {
        if completed.try_recv().is_ok() {
            unsafe { &mut *file_state.inner.get() }.completed.take();
        }
    }
    let file_path = &unsafe { &*file_state.inner.get() }.file_path;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .custom_flags(0x4000)
        .open(file_path)
        .await?;
    file.write_all(&buf[..]).await?;
    if file.metadata().await?.len() > MAX_FILE_SIZE
        && !unsafe { &mut *file_state.inner.get() }.completed.is_none()
    {
        let new_file_path = PathBuf::from(format!(
            "{}/seqnum-{}.out",
            CONFIG.server.append_dir,
            ID.fetch_add(1, Ordering::AcqRel)
        ));
        let (sender, receiver) = oneshot::channel();
        unsafe { &mut *file_state.inner.get() }.file_path = new_file_path.clone();
        unsafe { &mut *file_state.inner.get() }.completed = Some(receiver);
        let old_file_path = file_path.clone();
        let new_file_path = new_file_path;
        tokio::spawn(async move {
            let mut buf = [0u8; 24];
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&old_file_path)
                .await
                .unwrap();
            let mut map = AHashMap::new();
            while let Ok(_) = file.read_exact(&mut buf[..]).await {
                let (key, seq_num) = from_bytes(&buf[..]);
                map.entry(key)
                    .and_modify(|seqnum| {
                        if *seqnum < seq_num {
                            *seqnum = seq_num;
                        }
                    })
                    .or_insert(seq_num);
            }
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x4000)
                .open(&new_file_path)
                .await
                .unwrap();
            for (key, seqnum) in map {
                let mut buf = [0u8; 24];
                as_bytes(key, seqnum, &mut buf[..]);
                file.write_all(&buf[..]).await.unwrap();
            }
            tokio::fs::remove_file(&old_file_path).await.unwrap();
            sender.send(()).unwrap();
        });
    }
    // let state = match FILE.get() {
    //     Some(file) => file,
    //     None => {
    //         let file_path_str = format!(
    //             "{}/seqnum-{}.out",
    //             CONFIG.server.append_dir,
    //             ID.fetch_add(1, Ordering::AcqRel)
    //         );
    //         // we must block here to await file creation completed.
    //         let file = tokio::fs::OpenOptions::new()
    //             .create(true)
    //             .append(true)
    //             .custom_flags(0x4000)
    //             .open(&file_path_str).await.unwrap();
    //         let new_state = InnerStateMutable {
    //             inner: UnsafeCell::new(Some(InnerState {
    //                 file,
    //                 file_path: PathBuf::from(file_path_str),
    //                 file_path_rx: None,
    //             })),
    //         };
    //         FILE.get_or(|| new_state)
    //     }
    // };
    // let file = &mut unsafe { &mut *state.inner.get() }.as_mut().unwrap().file;
    // file.write_all(&buf[..]).await?;
    // let file = &unsafe { &*state.inner.get() }.as_ref().unwrap().file;
    // let file_path = &unsafe { &mut *state.inner.get() }
    //     .as_mut()
    //     .unwrap()
    //     .file_path;
    // let file_path_rx = &mut unsafe { &mut *state.inner.get() }
    //     .as_mut()
    //     .unwrap()
    //     .file_path_rx;
    // if file.metadata().await?.len() > MAX_FILE_SIZE && file_path_rx.is_none() {
    //     let temp_seqnum_file_path_str = format!(
    //         "{}/seqnum-temp-{}.out",
    //         CONFIG.server.append_dir,
    //         get_file_id(&file_path.file_name().unwrap().to_str().unwrap())
    //     );
    //     let temp_seqnum_file = tokio::fs::File::from_std(std::fs::OpenOptions::new()
    //         .create(true)
    //         .append(true)
    //         .custom_flags(0x4000)
    //         .open(&temp_seqnum_file_path_str).unwrap());
    //     let old_seqnum_file = unsafe { &mut *state.inner.get() }.take().unwrap();
    //     let (tx, rx) = oneshot::channel();
    //     let new_state = InnerState {
    //         file: temp_seqnum_file,
    //         file_path: PathBuf::from(temp_seqnum_file_path_str),
    //         file_path_rx: Some(rx),
    //     };
    //     unsafe { &mut *state.inner.get() }.replace(new_state);
    //     tokio::spawn(async move {
    //         let mut old_seqnum_file0 = tokio::fs::OpenOptions::new()
    //             .read(true)
    //             .open(&old_seqnum_file.file_path)
    //             .await.unwrap();
    //         let mut map = AHashMap::new();
    //         let mut archive_buf = [0u8; 24];
    //         while let Ok(_) = old_seqnum_file0.read_exact(&mut archive_buf[..]).await {
    //             let (key, seq_num) = from_bytes(&archive_buf[..]);
    //             map.entry(key)
    //                 .and_modify(|v| *v = seq_num)
    //                 .or_insert(seq_num);
    //         }
    //         let archive_seqnum_file_path_str = format!(
    //             "{}/seqnum-archive-{}.out",
    //             CONFIG.server.append_dir,
    //             get_file_id(
    //                 &old_seqnum_file
    //                     .file_path
    //                     .file_name()
    //                     .unwrap()
    //                     .to_str()
    //                     .unwrap()
    //             )
    //         );
    //         let mut archive_seqnum_file = tokio::fs::File::from_std(
    //             std::fs::OpenOptions::new()
    //                 .create(true)
    //                 .append(true)
    //                 .custom_flags(0x4000)
    //                 .open(&archive_seqnum_file_path_str).unwrap(),
    //         );
    //         for (key, seq_num) in map {
    //             as_bytes(key, seq_num, &mut archive_buf[..]);
    //             archive_seqnum_file.write_all(&archive_buf[..]).await?;
    //         }
    //         tokio::fs::remove_file(&old_seqnum_file.file_path).await?;
    //         _ = tx.send(PathBuf::from(archive_seqnum_file_path_str));
    //         Result::<()>::Ok(())
    //     });
    // }
    // let mut file = &mut unsafe { &mut *state.inner.get() }.as_mut().unwrap().file;
    // let file_path = &mut unsafe { &mut *state.inner.get() }
    //     .as_mut()
    //     .unwrap()
    //     .file_path;
    // let mut file_path_rx = &mut unsafe { &mut *state.inner.get() }
    //     .as_mut()
    //     .unwrap()
    //     .file_path_rx;
    // file.write_all(&buf[..]).await?;
    // if let Some(signal) = &mut file_path_rx {
    //     if let Ok(archive) = signal.try_recv() {
    //         let new_seqnum_file_path_str = format!(
    //             "{}/seqnum-new-{}.out",
    //             CONFIG.server.append_dir,
    //             ID.fetch_add(1, Ordering::AcqRel)
    //         );
    //         let mut new_seqnum_file = tokio::fs::File::from_std(
    //             std::fs::OpenOptions::new()
    //                 .create(true)
    //                 .append(true)
    //                 .custom_flags(0x4000) // O_DIRECT but macOS doesn't support it
    //                 .open(&new_seqnum_file_path_str)?,
    //         );
    //         let mut archive_seqnum_file = tokio::fs::OpenOptions::new()
    //             .read(true)
    //             .open(&archive)
    //             .await.unwrap();
    //         let mut temp_seqnum_file = tokio::fs::OpenOptions::new()
    //             .read(true)
    //             .open(&file_path)
    //             .await.unwrap();
    //         let mut copy_buf = [0u8; 24 * 1024];
    //         let mut index = 0;
    //         loop {
    //             match archive_seqnum_file.read(&mut copy_buf[index..]).await {
    //                 Ok(size) => {
    //                     if index == 0 {
    //                         break;
    //                     }
    //                     new_seqnum_file.write_all(&copy_buf[index..]).await?;
    //                     index += size;
    //                     if index == copy_buf.len() {
    //                         index = 0;
    //                     }
    //                 }
    //                 Err(e) => {
    //                     error!("read archive seqnum file error: {}", e);
    //                     break;
    //                 }
    //             }
    //         }
    //         loop {
    //             match temp_seqnum_file.read(&mut copy_buf[index..]).await {
    //                 Ok(size) => {
    //                     if index == 0 {
    //                         break;
    //                     }
    //                     new_seqnum_file.write_all(&copy_buf[index..]).await?;
    //                     index += size;
    //                     if index == copy_buf.len() {
    //                         index = 0;
    //                     }
    //                 }
    //                 Err(e) => {
    //                     error!("read temp seqnum file error: {}", e);
    //                     break;
    //                 }
    //             }
    //         }
    //         tokio::fs::remove_file(&archive).await?;
    //         tokio::fs::remove_file(&file_path).await?;
    //         let new_state = InnerState {
    //             file: new_seqnum_file,
    //             file_path: PathBuf::from(new_seqnum_file_path_str),
    //             file_path_rx: None,
    //         };
    //         unsafe { &mut *state.inner.get() }.replace(new_state);
    //     }
    // }
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

fn as_bytes(key: u128, seqnum: u64, buf: &mut [u8]) {
    // BigEndian::write_u128(&mut buf[0..16], key);
    // BigEndian::write_u64(&mut buf[16..24], seqnum);
    buf.copy_from_slice(
        &format!(
            "{:07}-{:07}:{:07}\n",
            (key >> 64) as u64,
            key as u64,
            seqnum
        )
        .as_bytes()[..],
    );
}

fn from_bytes(buf: &[u8]) -> (u128, u64) {
    // (
    //     BigEndian::read_u128(&buf[0..16]),
    //     BigEndian::read_u64(&buf[16..24]),
    // )
    (
        (String::from_utf8_lossy(&buf[0..7])
            .to_string()
            .parse::<u64>()
            .unwrap() as u128)
            << 64
            | (String::from_utf8_lossy(&buf[9..15])
                .to_string()
                .parse::<u64>()
                .unwrap() as u128),
        String::from_utf8_lossy(&buf[16..23])
            .to_string()
            .parse::<u64>()
            .unwrap(),
    )
}
