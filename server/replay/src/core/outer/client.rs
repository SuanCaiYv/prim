use std::sync::Arc;

use crate::config::CONFIG;
use anyhow::anyhow;
use common::entity::Msg;
use common::entity::ServerInfo;
use common::entity::Type;
use common::net::client::ClientConfig;
use common::net::{InnerSender, LenBuffer, MsgIO, OuterReceiver, ALPN_PRIM};
use common::util::default_bind_ip;
use common::Result;
use futures_util::StreamExt;
use quinn::{RecvStream, SendStream};
use tracing::error;

pub struct Client {
    config: Option<ClientConfig>,
    io_streams: Option<(SendStream, RecvStream)>,
    failed_sender: InnerSender,
    failed_receiver: Option<OuterReceiver>,
    len_buffer: Box<LenBuffer>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        let (sender, receiver) =
            tokio::sync::mpsc::channel(CONFIG.performance.max_sender_side_channel_size);
        Self {
            config: Some(config),
            io_streams: None,
            failed_sender: sender,
            failed_receiver: Some(receiver),
            len_buffer: Box::new([0_u8; 4]),
        }
    }
    pub async fn run(&mut self, server_info: ServerInfo) -> Result<()> {
        let ClientConfig {
            remote_address,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
            max_uni_streams,
            ..
        } = self.config.take().unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = quinn::Endpoint::client(default_bind_ip())?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut client_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(max_bi_streams)
            .max_concurrent_uni_streams(max_uni_streams)
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection {
            connection,
            mut uni_streams,
            ..
        } = new_connection;
        let io_streams = connection.open_bi().await?;
        let mut msg = Msg::raw_payload(&server_info.to_bytes());
        msg.set_type(Type::Auth);
        msg.set_sender(server_info.id as u64);
        msg.set_sender_node(server_info.id);
        self.req(Arc::new(msg)).await?;
        self.io_streams = Some(io_streams);
        let failed_stream = uni_streams.next().await;
        if let Some(timeout_stream) = failed_stream {
            if let Ok(mut timeout_stream) = timeout_stream {
                let mut buffer: Box<LenBuffer> = Box::new([0_u8; 4]);
                let failed_sender = &self.failed_sender;
                loop {
                    let msg = MsgIO::read_msg(&mut buffer, &mut timeout_stream).await;
                    if let Ok(msg) = msg {
                        let res = failed_sender.send(msg).await;
                        if let Err(e) = res {
                            return Err(anyhow!("error sending msg to timeout channel: {}", e));
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn req(&mut self, msg: Arc<Msg>) -> Result<Arc<Msg>> {
        let (send, recv) = self.io_streams.as_mut().unwrap();
        let res = MsgIO::write_msg(msg, send).await;
        if let Err(e) = res {
            return Err(anyhow!("error writing msg to io stream: {}", e));
        }
        let res_msg = MsgIO::read_msg(&mut self.len_buffer, recv).await;
        if let Err(e) = res_msg {
            return Err(anyhow!("error reading msg from io stream: {}", e));
        }
        Ok(res_msg.unwrap())
    }

    pub async fn set_failed_handler<F, R>(&mut self, on_failed: F)
    where
        F: Fn(Arc<Msg>) -> R + 'static + Sync + Send,
        R: std::future::Future<Output = Result<()>> + 'static + Send + Sync,
    {
        let mut failed_receiver = self.failed_receiver.take().unwrap();
        tokio::spawn(async move {
            loop {
                let msg = failed_receiver.recv().await;
                if let Some(msg) = msg {
                    let res = on_failed(msg).await;
                    if let Err(e) = res {
                        error!("error handling timeout: {}", e);
                    }
                }
            }
        });
    }
}
