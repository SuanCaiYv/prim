use std::{
    cell::UnsafeCell,
    io::Write,
    os::unix::prelude::OpenOptionsExt,
    path::PathBuf,
    pin::Pin,
    sync::{atomic::AtomicU64, Arc},
    task::Poll,
};

use ahash::AHashMap;

use byteorder::ByteOrder;
use futures_util::Future;
use lazy_static::lazy_static;
use thread_local::ThreadLocal;
use tokio::{
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::oneshot,
};

use lib::Result;

use crate::{config::CONFIG, service::get_seqnum_map};

pub(self) const MAX_FILE_SIZE: u64 = 1024;

lazy_static! {
    static ref ID: AtomicU64 = AtomicU64::new(0);
    static ref FILE1: ThreadLocal<InnerStateMutable1> = ThreadLocal::new();
    static ref FILE2: ThreadLocal<InnerStateMutable2> = ThreadLocal::new();
    static ref FILE3: ThreadLocal<InnerStateMutable3> = ThreadLocal::new();
}

pub(self) struct InnerStateMutable1 {
    inner: UnsafeCell<PathBuf>,
}

unsafe impl Send for InnerStateMutable1 {}

unsafe impl Sync for InnerStateMutable1 {}

pub(self) struct InnerStateMutable2 {
    // the reason we choose std not tokio is that,
    // tokio fs + thread local has poll_write bug that we can't resolve.
    // by the way, we acquire every writing operation must persistance on disk,
    // so we need to wait for it done before accept next writing operation,
    // so we don't need tokio's async write.
    inner: UnsafeCell<std::fs::File>,
}

unsafe impl Send for InnerStateMutable2 {}

unsafe impl Sync for InnerStateMutable2 {}

// impl InnerStateMutable2 {
//     pub(self) fn write<'a, 'b: 'a>(&'a self, buf: &'b [u8]) -> Writer<'a> {
//         let file = unsafe { &mut *self.inner.get() };
//         Writer { file, buf }
//     }
// }

pub(self) struct InnerStateMutable3 {
    inner: UnsafeCell<Option<oneshot::Receiver<()>>>,
}

struct Writer<'a> {
    file: &'a mut tokio::fs::File,
    buf: &'a [u8],
}

impl<'a> Future for Writer<'a> {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut buf = &self.buf[..];
        loop {
            match Pin::new(&mut self.file).poll_write(cx, buf) {
                Poll::Ready(Ok(n)) => {
                    if n == buf.len() {
                        return Poll::Ready(Ok(()));
                    } else {
                        buf = &buf[n..];
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

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
    let file_path = FILE1.get_or(|| {
        let file_path = format!(
            "{}/seqnum-{}",
            CONFIG.server.append_dir,
            ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        InnerStateMutable1 {
            inner: UnsafeCell::new(PathBuf::from(file_path)),
        }
    });
    let file = match FILE2.get() {
        Some(file) => file,
        None => {
            let path = unsafe { &*file_path.inner.get() }.clone();
            let file = std::fs::OpenOptions::new()
                .create(true)
                .custom_flags(0x0400)
                .append(true)
                .open(path)?;
            FILE2.get_or(|| InnerStateMutable2 {
                inner: UnsafeCell::new(file),
            })
        }
    };
    let file_rx = match FILE3.get() {
        Some(file_rx) => file_rx,
        None => FILE3.get_or(|| InnerStateMutable3 {
            inner: UnsafeCell::new(None),
        }),
    };
    let mut buf = [0u8; 24];
    as_bytes(key, seqnum, &mut buf[..]);
    let file0 = unsafe { &mut *file.inner.get() };
    file0.write_all(&buf[..])?;
    if file0.metadata()?.len() > MAX_FILE_SIZE && unsafe { &*file_rx.inner.get() }.is_none() {
        let new_file_path = format!(
            "{}/seqnum-{}",
            CONFIG.server.append_dir,
            ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        let new_file = std::fs::OpenOptions::new()
            .create(true)
            .custom_flags(0x0400)
            .append(true)
            .open(&new_file_path)?;
        let (tx, rx) = oneshot::channel();
        let old_file_path = unsafe { &*file_path.inner.get() }.clone();
        *unsafe { &mut *file_path.inner.get() } = new_file_path.clone().into();
        *unsafe { &mut *file.inner.get() } = new_file;
        *unsafe { &mut *file_rx.inner.get() } = Some(rx);
        tokio::spawn(async move {
            let mut buf = [0u8; 24];
            let mut old_file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(&old_file_path)
                .await
                .unwrap();
            let mut new_file = tokio::fs::OpenOptions::new()
                .create(true)
                .custom_flags(0x0400)
                .append(true)
                .open(&new_file_path)
                .await
                .unwrap();
            let mut map = AHashMap::new();
            while let Ok(_) = old_file.read_exact(&mut buf[..]).await {
                let (key, seq_num) = from_bytes(&buf[..]);
                map.entry(key)
                    .and_modify(|seqnum| {
                        if *seqnum < seq_num {
                            *seqnum = seq_num;
                        }
                    })
                    .or_insert(seq_num);
            }
            for (key, seqnum) in map {
                let mut buf = [0u8; 24];
                as_bytes(key, seqnum, &mut buf[..]);
                new_file.write_all(&buf[..]).await.unwrap();
            }
            tx.send(()).unwrap();
            println!("remove file: {:?} append to new file: {:?}", old_file_path, new_file_path);
            tokio::fs::remove_file(old_file_path).await.unwrap();
        });
    }
    if let Some(rx) = unsafe { &mut *file_rx.inner.get() }.as_mut() {
        if rx.try_recv().is_ok() {
            unsafe { &mut *file_rx.inner.get() }.take();
        }
    }
    Ok(())
}

fn as_bytes(key: u128, seqnum: u64, buf: &mut [u8]) {
    byteorder::BigEndian::write_u128(&mut buf[0..16], key);
    byteorder::BigEndian::write_u64(&mut buf[16..24], seqnum);
    // buf.copy_from_slice(
    //     &format!(
    //         "{:07}-{:07}:{:07}\n",
    //         (key >> 64) as u64,
    //         key as u64,
    //         seqnum
    //     )
    //     .as_bytes()[..],
    // );
}

fn from_bytes(buf: &[u8]) -> (u128, u64) {
    (
        byteorder::BigEndian::read_u128(&buf[0..16]),
        byteorder::BigEndian::read_u64(&buf[16..24]),
    )
    // (
    //     (String::from_utf8_lossy(&buf[0..7])
    //         .to_string()
    //         .parse::<u64>()
    //         .unwrap() as u128)
    //         << 64
    //         | (String::from_utf8_lossy(&buf[9..15])
    //             .to_string()
    //             .parse::<u64>()
    //             .unwrap() as u128),
    //     String::from_utf8_lossy(&buf[16..23])
    //         .to_string()
    //         .parse::<u64>()
    //         .unwrap(),
    // )
}
