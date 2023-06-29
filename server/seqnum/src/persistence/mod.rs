// use std::{
//     cell::UnsafeCell,
//     io::Write,
//     os::unix::prelude::OpenOptionsExt,
//     path::PathBuf,
//     pin::Pin,
//     sync::{atomic::AtomicU64, Arc},
//     task::Poll,
// };

// use ahash::AHashMap;
// use byteorder::ByteOrder;
// use futures::Future;
// use lazy_static::lazy_static;
// use lib::Result;
// use local_sync::oneshot;
// use thread_local::ThreadLocal;

// use crate::{config::CONFIG, service::get_seqnum_map};

// pub(self) const MAX_FILE_SIZE: u64 = 1024;

// lazy_static! {
//     static ref ID: AtomicU64 = AtomicU64::new(0);
//     static ref FILE1: ThreadLocal<InnerStateMutable1> = ThreadLocal::new();
//     static ref FILE2: ThreadLocal<InnerStateMutable2> = ThreadLocal::new();
//     static ref FILE3: ThreadLocal<InnerStateMutable3> = ThreadLocal::new();
//     static ref TEST0: ThreadLocal<InnerStateMutableT> = ThreadLocal::new();
// }

// pub(self) struct InnerStateMutable1 {
//     inner: UnsafeCell<PathBuf>,
// }

// unsafe impl Send for InnerStateMutable1 {}

// unsafe impl Sync for InnerStateMutable1 {}

// pub(self) struct InnerStateMutable2 {
//     inner: UnsafeCell<(monoio::fs::File, u64)>,
// }

// unsafe impl Send for InnerStateMutable2 {}

// unsafe impl Sync for InnerStateMutable2 {}

// // impl InnerStateMutable2 {
// //     pub(self) fn write<'a, 'b: 'a>(&'a self, buf: &'b [u8]) -> Writer<'a> {
// //         let file = unsafe { &mut *self.inner.get() };
// //         Writer { file, buf }
// //     }
// // }

// pub(self) struct InnerStateMutable3 {
//     inner: UnsafeCell<Option<oneshot::Receiver<()>>>,
// }

// unsafe impl Send for InnerStateMutable3 {}

// unsafe impl Sync for InnerStateMutable3 {}

// // struct Writer<'a> {
// //     file: &'a mut monoio::fs::File,
// //     buf: &'a [u8],
// // }

// // impl<'a> Future for Writer<'a> {
// //     type Output = Result<()>;

// //     fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
// //         let mut buf = &self.buf[..];
// //         loop {
// //             match Pin::new(&mut self.file).poll_write(cx, buf) {
// //                 Poll::Ready(Ok(n)) => {
// //                     if n == buf.len() {
// //                         return Poll::Ready(Ok(()));
// //                     } else {
// //                         buf = &buf[n..];
// //                     }
// //                 }
// //                 Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
// //                 Poll::Pending => return Poll::Pending,
// //             }
// //         }
// //     }
// // }



// pub(crate) async fn save(key: u128, seqnum: u64) -> Result<()> {
//     let file_path = FILE1.get_or(|| {
//         let file_path = format!(
//             "{}/seqnum-{}",
//             CONFIG.server.append_dir,
//             ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
//         );
//         InnerStateMutable1 {
//             inner: UnsafeCell::new(PathBuf::from(file_path)),
//         }
//     });
//     let file = match FILE2.get() {
//         Some(file) => file,
//         None => {
//             let path = unsafe { &*file_path.inner.get() }.clone();
//             let file = monoio::fs::OpenOptions::new()
//                 .create(true)
//                 .custom_flags(0x0400)
//                 .append(true)
//                 .open(path)
//                 .await?;
//             let file_len = std::fs::metadata(path).unwrap().len();
//             FILE2.get_or(|| InnerStateMutable2 {
//                 inner: UnsafeCell::new((file, file_len)),
//             })
//         }
//     };
//     let file_rx = match FILE3.get() {
//         Some(file_rx) => file_rx,
//         None => FILE3.get_or(|| InnerStateMutable3 {
//             inner: UnsafeCell::new(None),
//         }),
//     };
//     let mut buf = [0u8; 24];
//     as_bytes(key, seqnum, &mut buf[..]);
//     let file0 = unsafe { &mut *file.inner.get() };
//     let (res, _) = file0.0.write_all_at(buf.to_owned(), 0).await;
//     res?;
//     file0.1 += 24;
//     if file0.1 > MAX_FILE_SIZE && unsafe { &*file_rx.inner.get() }.is_none() {
//         let new_file_path = format!(
//             "{}/seqnum-{}",
//             CONFIG.server.append_dir,
//             ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
//         );
//         let new_file = monoio::fs::OpenOptions::new()
//             .create(true)
//             .custom_flags(0x0400)
//             .append(true)
//             .open(&new_file_path)
//             .await?;
//         let (tx, rx) = oneshot::channel();
//         let old_file_path = unsafe { &*file_path.inner.get() }.clone();
//         *unsafe { &mut *file_path.inner.get() } = new_file_path.clone().into();
//         *unsafe { &mut *file.inner.get() } = new_file;
//         *unsafe { &mut *file_rx.inner.get() } = Some(rx);
//         monoio::spawn(async move {
//             let mut buf = vec![0u8; 24];
//             let mut old_file = monoio::fs::OpenOptions::new()
//                 .read(true)
//                 .open(&old_file_path)
//                 .await
//                 .unwrap();
//             let mut new_file = monoio::fs::OpenOptions::new()
//                 .create(true)
//                 .custom_flags(0x0400)
//                 .append(true)
//                 .open(&new_file_path)
//                 .await
//                 .unwrap();
//             let mut map = AHashMap::new();
//             loop {
//                 let (res, buf) = old_file.read_exact_at(&mut buf[..], 0).await;
//                 if res.is_err() {
//                     break;
//                 }
//                 let (key, seq_num) = from_bytes(&buf[..]);
//                 map.entry(key)
//                     .and_modify(|seqnum| {
//                         if *seqnum < seq_num {
//                             *seqnum = seq_num;
//                         }
//                     })
//                     .or_insert(seq_num);
//             }
//             let mut buf = vec![0u8; 24];
//             for (key, seqnum) in map {
//                 as_bytes(key, seqnum, &mut buf[..]);
//                 let (res, buf) = new_file.write_all_at(buf, 0).await;
//                 if res.is_err() {
//                     break;
//                 }
//             }
//             tx.send(()).unwrap();
//             println!(
//                 "remove file: {:?} append to new file: {:?}",
//                 old_file_path, new_file_path
//             );
//             std::fs::remove_file(old_file_path).unwrap();
//         });
//     }
//     if let Some(rx) = unsafe { &mut *file_rx.inner.get() }.as_mut() {
//         if rx.try_recv().is_ok() {
//             unsafe { &mut *file_rx.inner.get() }.take();
//         }
//     }
//     Ok(())
// }

// fn as_bytes(key: u128, seqnum: u64, buf: &mut [u8]) {
//     byteorder::BigEndian::write_u128(&mut buf[0..16], key);
//     byteorder::BigEndian::write_u64(&mut buf[16..24], seqnum);
//     // buf.copy_from_slice(
//     //     &format!(
//     //         "{:07}-{:07}:{:07}\n",
//     //         (key >> 64) as u64,
//     //         key as u64,
//     //         seqnum
//     //     )
//     //     .as_bytes()[..],
//     // );
// }

// fn from_bytes(buf: &[u8]) -> (u128, u64) {
//     (
//         byteorder::BigEndian::read_u128(&buf[0..16]),
//         byteorder::BigEndian::read_u64(&buf[16..24]),
//     )
//     // (
//     //     (String::from_utf8_lossy(&buf[0..7])
//     //         .to_string()
//     //         .parse::<u64>()
//     //         .unwrap() as u128)
//     //         << 64
//     //         | (String::from_utf8_lossy(&buf[9..15])
//     //             .to_string()
//     //             .parse::<u64>()
//     //             .unwrap() as u128),
//     //     String::from_utf8_lossy(&buf[16..23])
//     //         .to_string()
//     //         .parse::<u64>()
//     //         .unwrap(),
//     // )
// }

// struct InnerStateMutableT {
//     inner: UnsafeCell<monoio::fs::File>,
// }

// unsafe impl Sync for InnerStateMutableT {}

// unsafe impl Send for InnerStateMutableT {}

// // impl InnerStateMutableT {
// //     pub(self) fn write<'a, 'b: 'a>(&'a self, buf: &'b [u8]) -> Writer<'a> {
// //         let file = unsafe { &mut *self.inner.get() };
// //         Writer { file, buf }
// //     }
// // }

// pub(crate) async fn test() {
//     let file_path = FILE1.get_or(|| {
//         let file_path = format!(
//             "{}/test-{}.txt",
//             CONFIG.server.append_dir,
//             ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
//         );
//         InnerStateMutable1 {
//             inner: UnsafeCell::new(PathBuf::from(file_path)),
//         }
//     });
//     let path = unsafe { &*file_path.inner.get() }.clone();
//     let file = match TEST0.get() {
//         Some(file) => file,
//         None => {
//             let file = monoio::fs::OpenOptions::new()
//                 .create(true)
//                 .custom_flags(0x0400)
//                 .append(true)
//                 .open(path)
//                 .await
//                 .unwrap();
//             TEST0.get_or(|| InnerStateMutableT {
//                 inner: UnsafeCell::new(file),
//             })
//         }
//     };
//     println!("thread id: {}", thread_id::get());
//     unsafe { &mut *file.inner.get() }
//         .write_all_at("12345678900987654321abc\n".as_bytes().to_vec(), 0)
//         .await;
// }
