use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use async_channel::{Receiver, Sender};
use futures_util::{AsyncWriteExt, StreamExt};
use quinn::{Connection, IncomingBiStreams, IncomingUniStreams, RecvStream, SendStream, VarInt};
use tokio::select;
use tracing::{debug, error, info, warn};

use crate::cache::get_redis_ops;
use crate::config::CONFIG;
use crate::core::{
    get_connection_map, get_status_map, Buffer, Handler, HandlerParameters, ALPN_PRIM,
    BODY_BUF_LENGTH,
};
use crate::entity::{msg, HEAD_LEN};
use crate::error;

use super::Result;

type HandlerList = Arc<Vec<Box<dyn Handler + Send + Sync + 'static>>>;

pub(super) struct Server {
    address: SocketAddr,
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
    max_connections: u32,
}

#[allow(unused)]
pub(super) struct ConnectionTask {
    /// provide some external information.
    #[allow(unused)]
    connection: Connection,
    bi_streams: IncomingBiStreams,
    /// for now, we just forbidden the use of unidirectional streams.
    /// It may by used in the future version.
    #[allow(unused)]
    uni_streams: IncomingUniStreams,
    handler_list: HandlerList,
}

impl ConnectionTask {
    #[allow(unused)]
    fn new(
        connection: Connection,
        bi_streams: IncomingBiStreams,
        uni_streams: IncomingUniStreams,
        handler_list: HandlerList,
    ) -> ConnectionTask {
        ConnectionTask {
            connection,
            bi_streams,
            uni_streams,
            handler_list,
        }
    }

    #[allow(unused)]
    async fn handle(&mut self) -> Result<()> {
        let (to, from) = async_channel::bounded(1024);
        // the first stream and first msg should be `auth` msg.
        // when the first work, any error should shutdown the connection
        if let Some(stream) = self.bi_streams.next().await {
            debug!("get first stream...");
            if let Ok(stream) = stream {
                debug!("get first stream successfully");
                let handler_list = self.handler_list.clone();
                let from = from.clone();
                let connection = self.connection.clone();
                tokio::spawn(async move {
                    let res = Self::first_stream_task(handler_list, from, to, stream).await;
                    if res.is_err() {
                        connection.close(1_u8.into(), b"first read error.");
                    }
                });
            } else {
                self.connection
                    .close(VarInt::from(1_u8), b"first read failed.");
                return Err(anyhow!("first stream and read fatal."));
            }
        } else {
            self.connection
                .close(VarInt::from(1_u8), "first read failed.".as_bytes());
            return Err(anyhow!("first stream and read fatal."));
        }
        while let Some(stream) = self.bi_streams.next().await {
            let stream = match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("connection closed.");
                    return Ok(());
                }
                Err(quinn::ConnectionError::ConnectionClosed { .. }) => {
                    info!("connection closed but by quic.");
                    return Ok(());
                }
                Err(quinn::ConnectionError::Reset) => {
                    info!("connection reset.");
                    return Ok(());
                }
                Err(quinn::ConnectionError::TransportError { .. }) => {
                    info!("connect by fake specification.");
                    return Ok(());
                }
                Err(quinn::ConnectionError::TimedOut) => {
                    info!("connection idle for too long time.");
                    return Ok(());
                }
                Err(quinn::ConnectionError::VersionMismatch) => {
                    info!("connect by unsupported protocol version.");
                    return Ok(());
                }
                Err(quinn::ConnectionError::LocallyClosed) => {
                    info!("local server fatal.");
                    return Ok(());
                }
                Ok(ok) => ok,
            };
            let handler_list = self.handler_list.clone();
            let from = from.clone();
            tokio::spawn(async move {
                let res = Self::new_stream_task(handler_list, from, stream).await;
            });
        }
        self.connection
            .close(VarInt::from(0_u8), "connection done.".as_bytes());
        Ok(())
    }

    #[allow(unused)]
    async fn first_stream_task(
        handler_list: HandlerList,
        mut from: Receiver<msg::Msg>,
        to: Sender<msg::Msg>,
        (mut send, mut recv): (SendStream, RecvStream),
    ) -> Result<()> {
        let mut parameters = HandlerParameters {
            buffer: Buffer {
                head_buf: [0; HEAD_LEN],
                body_buf: Box::new([0; BODY_BUF_LENGTH]),
            },
            stream: (send, recv),
            outer_stream: from,
            connection_map: get_connection_map(),
            status_map: get_status_map(),
            redis_ops: get_redis_ops().await,
        };
        debug!("first read task start.");
        let auth = &handler_list[0];
        let mut msg = Self::read_msg(&mut parameters.buffer, &mut parameters.stream.1).await?;
        debug!("first read task read msg: {}", msg);
        let res = auth.handle_function(&mut msg, &mut parameters).await;
        debug!("first read task handle result: {:?}", res);
        if let Ok(success) = res {
            parameters.connection_map.insert(msg.head.sender, to);
            Self::write_msg(
                &msg.generate_ack(msg.head.timestamp),
                &mut parameters.stream.0,
            )
            .await?;
        } else {
            Self::write_msg(
                &msg::Msg::err_msg_str(0, msg.head.sender, "auth failed."),
                &mut parameters.stream.0,
            )
            .await?;
            error!("first read task auth failed: {}", res.err().unwrap());
            return Err(anyhow!("auth failed."));
        }
        Self::epoll_stream(handler_list, &mut parameters).await?;
        parameters.stream.0.finish().await?;
        parameters.outer_stream.close();
        Ok(())
    }

    #[allow(unused)]
    async fn new_stream_task(
        handler_list: HandlerList,
        mut from: Receiver<msg::Msg>,
        (mut send, mut recv): (SendStream, RecvStream),
    ) -> Result<()> {
        let mut parameters = HandlerParameters {
            buffer: Buffer {
                head_buf: [0; HEAD_LEN],
                body_buf: Box::new([0; BODY_BUF_LENGTH]),
            },
            stream: (send, recv),
            outer_stream: from,
            connection_map: get_connection_map(),
            status_map: get_status_map(),
            redis_ops: get_redis_ops().await,
        };
        Self::epoll_stream(handler_list, &mut parameters).await?;
        parameters.stream.0.finish().await?;
        parameters.outer_stream.close();
        Ok(())
    }

    #[allow(unused)]
    #[inline]
    async fn epoll_stream(
        handler_list: HandlerList,
        parameters: &mut HandlerParameters,
    ) -> Result<()> {
        loop {
            select! {
                msg = parameters.outer_stream.recv() => {
                    if let Ok(mut msg) = msg {
                        let res = Self::write_msg(&mut msg, &mut parameters.stream.0).await;
                        if let Err(e) = res {
                            error!("write msg error: {}", e);
                            continue;
                        }
                    } else {
                        info!("outer stream closed.");
                        break;
                    }
                },
                msg = Self::read_msg(&mut parameters.buffer, &mut parameters.stream.1) => {
                    if let Ok(mut msg) = msg {
                        let res = Self::handle_msg(&handler_list, &mut msg, parameters).await;
                        if let Err(e) = res {
                            error!("handle msg error: {}", e);
                            continue
                        }
                    } else {
                        let err = msg.err().unwrap();
                        if let Ok(error::CrashError::ShouldCrash(cause)) = err.downcast::<error::CrashError>() {
                            warn!("should crash: {}", cause);
                            break
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(unused)]
    #[inline]
    async fn handle_msg(
        handler_list: &HandlerList,
        msg: &mut msg::Msg,
        parameters: &mut HandlerParameters,
    ) -> Result<()> {
        let mut res_msg = None;
        for handler in handler_list.iter() {
            let res = handler.handle_function(msg, parameters).await;
            if let Ok(success) = res {
                res_msg = Some(success);
            } else {
                let err = res.err().unwrap().downcast::<error::HandlerError>();
                if err.is_err() {
                    error!("unhandled error: {}", err.as_ref().err().unwrap());
                    return Err(anyhow!("unhandled error: {}", err.as_ref().err().unwrap()));
                }
                match err.unwrap() {
                    error::HandlerError::NotMine => {
                        continue;
                    }
                    error::HandlerError::Auth { .. } => {
                        let msg = msg::Msg::err_msg_str(0, msg.head.sender, "auth failed.");
                        res_msg = Some(msg);
                    }
                }
            }
        }
        Self::write_msg(&res_msg.unwrap(), &mut parameters.stream.0).await?;
        Ok(())
    }

    #[allow(unused)]
    #[inline]
    pub(super) async fn read_msg(buffer: &mut Buffer, recv: &mut RecvStream) -> Result<msg::Msg> {
        let readable = recv.read(&mut buffer.head_buf).await?;
        if let None = readable {
            return Err(anyhow!(error::CrashError::ShouldCrash(
                "stream finished.".to_string()
            )));
        }
        let readable = readable.unwrap();
        if readable != HEAD_LEN {
            return Err(anyhow!(error::MessageError::ReadBodyError(format!(
                "head length {} unexpected.",
                readable
            ))));
        }
        let head = msg::Head::from(&buffer.head_buf[..]);
        if head.length == 0 {
            let msg = msg::Msg {
                head,
                payload: Vec::new(),
            };
            return Ok(msg);
        }
        let size = recv
            .read(&mut buffer.body_buf[0..head.length as usize])
            .await?;
        if let None = size {
            return Err(anyhow!(error::CrashError::ShouldCrash(
                "stream finished.".to_string()
            )));
        }
        let size = size.unwrap();
        if size != head.length as usize {
            return Err(anyhow!(error::MessageError::ReadBodyError(format!(
                "body length {} unexpected with {}.",
                size, head.length
            ))));
        }
        let msg = msg::Msg {
            head,
            payload: buffer.body_buf[..size].to_vec(),
        };
        Ok(msg)
    }

    #[allow(unused)]
    #[inline]
    pub(super) async fn write_msg(msg: &msg::Msg, send: &mut SendStream) -> Result<()> {
        let bytes = msg.as_bytes();
        let slice = bytes.as_slice();
        let mut len = slice.len();
        while len > 0 {
            let n = send.write(slice).await?;
            len -= n;
        }
        debug!("write: {}", msg);
        send.flush().await?;
        Ok(())
    }
}

impl Server {
    pub fn new() -> Self {
        Server {
            address: CONFIG.server.address,
            cert: CONFIG.server.cert.clone(),
            key: CONFIG.server.key.clone(),
            max_connections: CONFIG.server.max_connections,
        }
    }

    pub async fn run(self) -> Result<()> {
        // deconstruct Server
        let Server {
            address,
            cert,
            key,
            max_connections,
        } = self;
        // set crypto for server
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
        // set custom alpn protocol
        server_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        // set max concurrent connections
        server_config.concurrent_connections(max_connections);
        server_config.use_retry(true);
        // set quic transport parameters
        Arc::get_mut(&mut server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(8_u8.into())
            .max_concurrent_uni_streams(8_u8.into())
            .keep_alive_interval(Some(Duration::from_millis(3000)))
            .max_idle_timeout(Some(quinn::IdleTimeout::from(quinn::VarInt::from_u32(
                15 * 3000,
            ))));
        let (endpoint, mut incoming) = quinn::Endpoint::server(server_config, address)?;
        // set handler list
        let mut handler_list: HandlerList = HandlerList::new(Vec::new());
        Arc::get_mut(&mut handler_list)
            .unwrap()
            .push(Box::new(super::handler::auth::Auth {}));
        Arc::get_mut(&mut handler_list)
            .unwrap()
            .push(Box::new(super::handler::echo::Echo {}));
        while let Some(conn) = incoming.next().await {
            let quinn::NewConnection {
                connection,
                bi_streams,
                uni_streams,
                ..
            } = conn.await?;
            let mut handler =
                ConnectionTask::new(connection, bi_streams, uni_streams, handler_list.clone());
            info!("new connection established.");
            tokio::spawn(async move {
                let _ = handler.handle().await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() {}
}
