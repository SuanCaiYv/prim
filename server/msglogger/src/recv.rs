use std::{
    fs,
    time::{Duration, Instant},
};

use lib::entity::{Head, Msg, Type, HEAD_LEN};
use monoio::{
    io::{AsyncReadRentExt, AsyncWriteRentExt},
    net::{UnixDatagram, UnixListener, UnixStream},
};
use tracing::{error, info};

pub(crate) async fn start_recv() {
    if let Err(e) = fs::remove_file("/tmp/msglogger.sock") {
        error!("failed to remove file: {:?}", e);
    }
    let listener = UnixListener::bind("/tmp/msglogger.sock");
    if let Err(e) = listener {
        error!("failed to bind listener: {:?}", e);
        return;
    }
    let listener = listener.unwrap();
    loop {
        let (mut stream, addr) = listener.accept().await.unwrap();
        info!("accepted connection from {:?}", addr);
        monoio::spawn(async move {
            println!("aaa");
        });
    }
}

pub(crate) async fn test_recv() {
    let stream = UnixStream::connect("/tmp/msglogger.sock").await;
    info!("stream: {:?}", stream);
    if let Err(e) = stream {
        error!("failed to connect to stream: {:?}", e);
        return;
    }
    let mut stream = stream.unwrap();
    let mut msg = Msg::raw(0, 0, 0, vec![0u8; 128].as_slice());
    msg.set_type(Type::RemoteInvoke);
    let mut buffer: Box<[u8; HEAD_LEN]> = Box::new([0; HEAD_LEN]);
    let t = Instant::now();
    let n = 100;
    for _ in 0..n {
        stream
            .write_all(msg.as_slice()[0..HEAD_LEN].to_owned())
            .await;
        stream.read_u8().await.unwrap();
    }
    println!("time: {:?}", t.elapsed().as_nanos() / n as u128);
}

// pub(crate) async fn start_recv1() {
//     if let Err(e) = fs::remove_file("/tmp/msglogger_rx.sock") {
//         error!("failed to remove file: {:?}", e);
//     }
//     let recv = UnixDatagram::bind("/tmp/msglogger_rx.sock");
//     if let Err(e) = recv {
//         error!("failed to bind listener: {:?}", e);
//         return;
//     }
//     let recv = recv.unwrap();
//     monoio::time::sleep(Duration::from_millis(1000)).await;
//     let send = UnixDatagram::connect("/tmp/msglogger_tx.sock").await.unwrap();
//     let mut buffer: Box<[u8; HEAD_LEN]> = Box::new([0; HEAD_LEN]);
//     let mut resp = Msg::empty();
//     resp.set_type(Type::Ack);
//     let mut bytes = 0;
//     let mut sum: usize = 0;
//     let mut res;
//     loop {
//         (res, buffer) = recv.read_exact(buffer).await;
//         send.write_all(vec![b'1']).await;
//         if sum == 1_00_000 * HEAD_LEN {
//             println!("sum: {}", sum);
//             break;
//         }
//         // let mut head = Head::from(&buffer[..]);
//         // let mut req = Msg::pre_alloc(&mut head);
//         // bytes = 0;
//         // let body = req.as_mut_body();
//         // loop {
//         //     match stream.recv(&mut body[bytes..]).await {
//         //         Ok(n) => {
//         //             bytes += n;
//         //             if bytes == body.len() {
//         //                 break;
//         //             }
//         //         }
//         //         Err(_) => {
//         //             error!("failed to read body from stream.");
//         //             break;
//         //         }
//         //     };
//         // }
//         // bytes = 0;
//         // match stream.send(&resp.as_slice()[bytes..]).await {
//         //     Ok(n) => {
//         //         bytes += n;
//         //         if bytes == resp.0.len() {
//         //             break;
//         //         }
//         //     },
//         //     Err(_) => {
//         //         error!("failed to write ack to stream.");
//         //         break;
//         //     },
//         // }
//     }
// }

// pub(crate) async fn test_recv1() {
//     if let Err(e) = fs::remove_file("/tmp/msglogger_tx.sock") {
//         error!("failed to remove file: {:?}", e);
//     }
//     let stream = UnixDatagram::bind("/tmp/msglogger_tx.sock");
//     if let Err(e) = stream {
//         error!("failed to bind listener: {:?}", e);
//         return;
//     }
//     let stream = stream.unwrap();
//     monoio::time::sleep(Duration::from_millis(100)).await;
//     stream.connect("/tmp/msglogger_rx.sock").unwrap();
//     let mut req = Msg::raw(0, 0, 0, vec![0u8; 128].as_slice());
//     req.set_type(Type::RemoteInvoke);
//     let mut buffer: [u8; HEAD_LEN] = [0; HEAD_LEN];
//     let t = Instant::now();
//     let n = 1_000_000;
//     let mut bytes = 0;
//     for _ in 0..n {
//         bytes = 0;
//         loop {
//             match stream.send(&req.as_slice()[bytes..HEAD_LEN]).await {
//                 Ok(n) => {
//                     bytes += n;
//                     if bytes == HEAD_LEN {
//                         break;
//                     }
//                 }
//                 Err(e) => {
//                     error!("failed to write head to stream. {}", e);
//                     break;
//                 }
//             };
//         }
//         stream.recv(&mut buffer[..1]).await.unwrap();
//         // loop {
//         //     match stream.send(&req.as_slice()[bytes..]).await {
//         //         Ok(n) => {
//         //             bytes += n;
//         //             if bytes == req.0.len() {
//         //                 break;
//         //             }
//         //         }
//         //         Err(e) => {
//         //             error!("failed to write body to stream. {}", e);
//         //             break;
//         //         }
//         //     };
//         // }
//         // bytes = 0;
//         // loop {
//         //     match stream.recv(&mut buffer[bytes..]).await {
//         //         Ok(n) => {
//         //             bytes += n;
//         //             if bytes == HEAD_LEN {
//         //                 break;
//         //             }
//         //         },
//         //         Err(_) => {
//         //             error!("failed to read head from stream.");
//         //             break;
//         //         },
//         //     };
//         // }
//         // let mut head = Head::from(&buffer[..]);
//         // let mut resp = Msg::pre_alloc(&mut head);
//         // bytes = 0;
//         // let body = resp.as_mut_body();
//         // loop {
//         //     match stream.recv(&mut body[bytes..]).await {
//         //         Ok(n) => {
//         //             bytes += n;
//         //             if bytes == body.len() {
//         //                 break;
//         //             }
//         //         },
//         //         Err(_) => {
//         //             error!("failed to read body from stream.");
//         //             break;
//         //         },
//         //     };
//         // }
//         // if resp.typ() != Type::Ack {
//         //     error!("failed to get ack.");
//         //     break;
//         // }
//     }
//     println!("time: {:?}", t.elapsed().as_nanos() / n as u128);
// }
