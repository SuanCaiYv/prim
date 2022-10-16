use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::anyhow;
use async_channel::{Receiver, Sender};
use futures_util::StreamExt;
use quinn::{
    Connection, IncomingBiStreams, IncomingUniStreams, ReadExactError, RecvStream, SendStream,
    VarInt,
};
use tokio::select;
use tracing::{debug, error, info, warn};

use crate::cache::get_redis_ops;
use crate::config::CONFIG;
use crate::core::{
    get_connection_map, get_status_map, Handler, HandlerParameters, LenBuffer, ALPN_PRIM, BODY_SIZE,
};
use common::entity::{Msg, HEAD_LEN};

use super::Result;

type HandlerList = Arc<Vec<Box<dyn Handler + Send + Sync + 'static>>>;

pub(super) struct Server {
    // todo merge some fileds with config
    address: SocketAddr,
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
    max_connections: u32,
}

/// provide some external information.
#[allow(unused)]
pub(super) struct ConnectionTask {
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
            if let Ok(stream) = stream {
                debug!("get first stream successfully");
                let handler_list = self.handler_list.clone();
                let from = from.clone();
                let mut parameters = HandlerParameters {
                    buffer: [0; 4],
                    stream,
                    outer_stream: from,
                    connection_map: get_connection_map(),
                    status_map: get_status_map(),
                    redis_ops: get_redis_ops().await,
                };
                let res = Self::first_read(&handler_list, &mut parameters, to).await;
                if res.is_err() {
                    self.connection
                        .close(VarInt::from(1_u8), b"first read failed.");
                    return Err(anyhow!("first read fatal."));
                }
                tokio::spawn(async move {
                    let res = Self::first_stream_task(handler_list, parameters).await;
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
                    info!("the peer close the connection.");
                    break;
                }
                Err(quinn::ConnectionError::ConnectionClosed { .. }) => {
                    info!("the peer close the connection but by quic.");
                    break;
                }
                Err(quinn::ConnectionError::Reset) => {
                    info!("connection reset.");
                    break;
                }
                Err(quinn::ConnectionError::TransportError { .. }) => {
                    warn!("connect by fake specification.");
                    break;
                }
                Err(quinn::ConnectionError::TimedOut) => {
                    warn!("connection idle for too long time.");
                    break;
                }
                Err(quinn::ConnectionError::VersionMismatch) => {
                    warn!("connect by unsupported protocol version.");
                    break;
                }
                Err(quinn::ConnectionError::LocallyClosed) => {
                    warn!("local server fatal.");
                    break;
                }
                Ok(ok) => ok,
            };
            let handler_list = self.handler_list.clone();
            let from = from.clone();
            tokio::spawn(async move {
                let res = Self::new_stream_task(handler_list, from, stream).await;
            });
        }
        // no more streams arrived, so this connection should be closed normally.
        self.connection
            .close(VarInt::from(0_u8), "connection done.".as_bytes());
        info!("connection done.");
        Ok(())
    }

    /// this method return an error means the connection is not authed.
    #[inline]
    async fn first_read(
        handler_list: &HandlerList,
        parameters: &mut HandlerParameters,
        to: Sender<Msg>,
    ) -> Result<()> {
        let auth = &handler_list[0];
        let mut msg = Self::read_msg(&mut parameters.buffer, &mut parameters.stream.1).await?;
        debug!("first read task read msg: {}", msg);
        let res = auth.handle_function(&mut msg, parameters).await;
        if let Ok(_) = res {
            parameters.connection_map.insert(msg.sender(), to);
            Self::write_msg(
                &mut msg.generate_ack(msg.timestamp()),
                &mut parameters.stream.0,
            )
            .await?;
        } else {
            // auth failed, so close the outer connection.
            to.close();
            Self::write_msg(
                &mut Msg::err_msg_str(0, msg.sender(), "auth failed."),
                &mut parameters.stream.0,
            )
            .await?;
            // give that error response and finish the stream.
            let _ = parameters.stream.0.finish().await;
            info!("first read with auth failed: {}", res.err().unwrap());
            return Err(anyhow!("auth failed."));
        }
        Ok(())
    }

    #[inline]
    async fn first_stream_task(
        handler_list: HandlerList,
        mut parameters: HandlerParameters,
    ) -> Result<()> {
        Self::epoll_stream(handler_list, &mut parameters).await?;
        Ok(())
    }

    /// this method never return errors.
    #[allow(unused)]
    async fn new_stream_task(
        handler_list: HandlerList,
        mut from: Receiver<Msg>,
        (mut send, mut recv): (SendStream, RecvStream),
    ) -> Result<()> {
        let mut parameters = HandlerParameters {
            buffer: [0; 4],
            stream: (send, recv),
            outer_stream: from,
            connection_map: get_connection_map(),
            status_map: get_status_map(),
            redis_ops: get_redis_ops().await,
        };
        Self::epoll_stream(handler_list, &mut parameters).await?;
        Ok(())
    }

    /// this method will never return an error. when it returned, that means this stream should be closed.
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
                        if res.is_err() {
                            break;
                        }
                    } else {
                        info!("outer stream closed.");
                        break;
                    }
                },
                msg = Self::read_msg(&mut parameters.buffer, &mut parameters.stream.1) => {
                    if let Ok(mut msg) = msg {
                        let res = Self::handle_msg(&handler_list, &mut msg, parameters).await;
                        if res.is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        parameters.stream.0.finish().await?;
        Ok(())
    }

    /// the only error returned indicates that the stream is closed.
    #[allow(unused)]
    #[inline]
    async fn handle_msg(
        handler_list: &HandlerList,
        msg: &mut Msg,
        parameters: &mut HandlerParameters,
    ) -> Result<()> {
        let mut res_msg = None;
        for handler in handler_list.iter() {
            let res = handler.handle_function(msg, parameters).await;
            if let Ok(success) = res {
                res_msg = Some(success);
            } else {
                let err = res.err().unwrap().downcast::<crate::error::HandlerError>();
                if err.is_err() {
                    error!("unhandled error: {}", err.as_ref().err().unwrap());
                    continue;
                }
                match err.unwrap() {
                    crate::error::HandlerError::NotMine => {
                        continue;
                    }
                    crate::error::HandlerError::Auth { .. } => {
                        let msg = Msg::err_msg_str(0, msg.sender(), "auth failed.");
                        res_msg = Some(msg);
                        break;
                    }
                }
            }
        }
        if res_msg.is_none() {
            let msg = Msg::err_msg_str(0, msg.sender(), "no handler found.");
            res_msg = Some(msg);
        }
        Self::write_msg(&mut res_msg.unwrap(), &mut parameters.stream.0).await?;
        Ok(())
    }

    /// the only error returned should cause the stream crashed.
    #[allow(unused)]
    #[inline]
    pub(super) async fn read_msg(buffer: &mut LenBuffer, recv: &mut RecvStream) -> Result<Msg> {
        let readable = recv.read_exact(&mut buffer[..]).await;
        match readable {
            Ok(_) => {}
            Err(e) => {
                return match e {
                    ReadExactError::FinishedEarly => {
                        info!("stream finished.");
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "stream finished.".to_string()
                        )))
                    }
                    ReadExactError::ReadError(e) => {
                        warn!("read stream error: {:?}", e);
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "read stream error.".to_string()
                        )))
                    }
                }
            }
        }
        let extension_size = Msg::read_u16(&buffer[0..2]);
        let payload_size = Msg::read_u16(&buffer[2..4]);
        if (payload_size + extension_size) as usize > BODY_SIZE {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut msg = Msg::pre_alloc(payload_size, extension_size);
        msg.update_payload_length(payload_size);
        msg.update_extension_length(extension_size);
        let size = recv
            .read_exact(
                &mut (msg.as_mut_slice()
                    [4..(HEAD_LEN + extension_size as usize + payload_size as usize)]),
            )
            .await;
        match size {
            Ok(_) => {}
            Err(e) => {
                return match e {
                    ReadExactError::FinishedEarly => {
                        info!("stream finished.");
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "stream finished.".to_string()
                        )))
                    }
                    ReadExactError::ReadError(e) => {
                        warn!("read stream error: {:?}", e);
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "read stream error.".to_string()
                        )))
                    }
                }
            }
        }
        Ok(msg)
    }

    /// the only error returned should cause the stream crashed.
    /// and this method will automatically finish the stream.
    #[allow(unused)]
    #[inline]
    pub(super) async fn write_msg(msg: &mut Msg, send: &mut SendStream) -> Result<()> {
        let res = send.write_all(msg.as_bytes().as_slice()).await;
        if let Err(e) = res {
            send.finish().await;
            warn!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
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
            .max_concurrent_bidi_streams(CONFIG.transport.max_bi_streams)
            .max_concurrent_uni_streams(CONFIG.transport.max_uni_streams)
            // the keep-alive interval should set on client.
            // todo address migration and keep-alive.
            .max_idle_timeout(Some(quinn::IdleTimeout::from(CONFIG.transport.connection_idle_timeout)));
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
