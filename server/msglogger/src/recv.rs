use std::time::Instant;

use lib::entity::{Head, Msg, Type, HEAD_LEN};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixDatagram, UnixListener, UnixStream},
};
use tracing::error;

pub(crate) async fn start_recv() {
    if let Err(e) = fs::remove_file("/tmp/msglogger.sock").await {
        error!("failed to remove file: {:?}", e);
    }
    let listener = UnixListener::bind("/tmp/msglogger.sock");
    if let Err(e) = listener {
        error!("failed to bind listener: {:?}", e);
        return;
    }
    let listener = listener.unwrap();
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut buffer: [u8; HEAD_LEN] = [0; HEAD_LEN];
            let mut ack_msg = Msg::empty();
            ack_msg.set_type(Type::Ack);
            loop {
                match stream.read_exact(&mut buffer).await {
                    Ok(_) => {}
                    Err(_) => {
                        error!("failed to read head from stream.");
                        break;
                    }
                };
                let mut head = Head::from(&buffer[..]);
                let mut msg = Msg::pre_alloc(&mut head);
                match stream.read_exact(msg.as_mut_body()).await {
                    Ok(_) => {}
                    Err(_) => {
                        error!("failed to read body from stream.");
                        break;
                    }
                }
                match stream.write_all(ack_msg.as_slice()).await {
                    Ok(_) => {
                        if let Err(_) = stream.flush().await {
                            error!("failed to flush stream.");
                            break;
                        }
                    }
                    Err(_) => {
                        error!("failed to write ack to stream.");
                        break;
                    }
                }
            }
        });
    }
}

pub(crate) async fn test_recv() {
    for _ in 0..8 {
        tokio::spawn(async move {
            let stream = UnixStream::connect("/tmp/msglogger.sock").await;
            if let Err(e) = stream {
                error!("failed to connect to stream: {:?}", e);
                return;
            }
            let mut stream = stream.unwrap();
            let mut msg = Msg::raw(0, 0, 0, vec![0u8; 128].as_slice());
            msg.set_type(Type::RemoteInvoke);
            let mut buffer: [u8; HEAD_LEN] = [0; HEAD_LEN];
            let t = Instant::now();
            let n = 125_000;
            for _ in 0..n {
                stream.write_all(msg.as_slice()).await.unwrap();
                stream.read_exact(&mut buffer).await.unwrap();
                let mut head = Head::from(&buffer[..]);
                let mut ack = Msg::pre_alloc(&mut head);
                stream.read_exact(ack.as_mut_body()).await.unwrap();
                if ack.typ() != Type::Ack {
                    error!("failed to get ack.");
                    break;
                }
            }
            println!("time: {:?}", t.elapsed());
        });
    }
}

pub(crate) async fn start_recv1() {
    if let Err(e) = fs::remove_file("/tmp/msglogger.sock").await {
        error!("failed to remove file: {:?}", e);
    }
    let stream = UnixDatagram::bind("/tmp/msglogger.sock");
    if let Err(e) = stream {
        error!("failed to bind listener: {:?}", e);
        return;
    }
    let stream = stream.unwrap();
    let mut buffer: [u8; HEAD_LEN] = [0; HEAD_LEN];
    let mut resp = Msg::empty();
    resp.set_type(Type::Ack);
    let mut bytes = 0;
    loop {
        bytes = 0;
        loop {
            match stream.recv(&mut buffer[bytes..]).await {
                Ok(n) => {
                    bytes += n;
                    if bytes == HEAD_LEN {
                        break;
                    }
                }
                Err(_) => {
                    error!("failed to read head from stream.");
                    break;
                }
            };
        }
        let mut head = Head::from(&buffer[..]);
        let mut req = Msg::pre_alloc(&mut head);
        bytes = 0;
        let body = req.as_mut_body();
        loop {
            match stream.recv(&mut body[bytes..]).await {
                Ok(n) => {
                    bytes += n;
                    if bytes == body.len() {
                        break;
                    }
                }
                Err(_) => {
                    error!("failed to read body from stream.");
                    break;
                }
            };
        }
        // bytes = 0;
        // match stream.send(&resp.as_slice()[bytes..]).await {
        //     Ok(n) => {
        //         bytes += n;
        //         if bytes == resp.0.len() {
        //             break;
        //         }
        //     },
        //     Err(_) => {
        //         error!("failed to write ack to stream.");
        //         break;
        //     },
        // }
    }
}

pub(crate) async fn test_recv1() {
    if let Err(e) = fs::remove_file("/tmp/msglogger1.sock").await {
        error!("failed to remove file: {:?}", e);
    }
    let stream = UnixDatagram::bind("/tmp/msglogger1.sock");
    if let Err(e) = stream {
        error!("failed to bind listener: {:?}", e);
        return;
    }
    let stream = stream.unwrap();
    stream.connect("/tmp/msglogger.sock").unwrap();
    let mut req = Msg::raw(0, 0, 0, vec![0u8; 128].as_slice());
    req.set_type(Type::RemoteInvoke);
    let mut buffer: [u8; HEAD_LEN] = [0; HEAD_LEN];
    let t = Instant::now();
    let n = 100_000;
    let mut bytes = 0;
    for _ in 0..n {
        bytes = 0;
        loop {
            match stream.send(&req.as_slice()[bytes..HEAD_LEN]).await {
                Ok(n) => {
                    bytes += n;
                    if bytes == HEAD_LEN {
                        break;
                    }
                }
                Err(e) => {
                    error!("failed to write head to stream. {}", e);
                    break;
                }
            };
        }
        loop {
            match stream.send(&req.as_slice()[bytes..]).await {
                Ok(n) => {
                    bytes += n;
                    if bytes == req.0.len() {
                        break;
                    }
                }
                Err(e) => {
                    error!("failed to write body to stream. {}", e);
                    break;
                }
            };
        }
        // bytes = 0;
        // loop {
        //     match stream.recv(&mut buffer[bytes..]).await {
        //         Ok(n) => {
        //             bytes += n;
        //             if bytes == HEAD_LEN {
        //                 break;
        //             }
        //         },
        //         Err(_) => {
        //             error!("failed to read head from stream.");
        //             break;
        //         },
        //     };
        // }
        // let mut head = Head::from(&buffer[..]);
        // let mut resp = Msg::pre_alloc(&mut head);
        // bytes = 0;
        // let body = resp.as_mut_body();
        // loop {
        //     match stream.recv(&mut body[bytes..]).await {
        //         Ok(n) => {
        //             bytes += n;
        //             if bytes == body.len() {
        //                 break;
        //             }
        //         },
        //         Err(_) => {
        //             error!("failed to read body from stream.");
        //             break;
        //         },
        //     };
        // }
        // if resp.typ() != Type::Ack {
        //     error!("failed to get ack.");
        //     break;
        // }
    }
    println!("time: {:?}", t.elapsed().as_nanos() / n as u128);
}
