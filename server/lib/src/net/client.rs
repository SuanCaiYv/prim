use std::pin::Pin;
use std::sync::atomic::{AtomicU16, AtomicU64, AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use std::{net::SocketAddr, sync::Arc, time::Duration};

use crate::entity::ReqwestMsg;
use crate::net::ReqwestMsgIOUtil;
use crate::{
    entity::{Msg, ServerInfo, Type},
    Result,
};

use ahash::AHashMap;
use anyhow::anyhow;

use futures_util::future::BoxFuture;
use futures_util::Future;
use quinn::{Connection, Endpoint};
use tokio::{io::split, net::TcpStream, select};
use tokio_rustls::{client::TlsStream, TlsConnector};
use tracing::error;

use super::{
    MsgIOTimeoutWrapper, MsgIOTlsClientTimeoutWrapper, MsgIOWrapper, MsgMpmcReceiver,
    MsgMpmcSender, MsgMpscReceiver, MsgMpscSender, ALPN_PRIM,
};

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub remote_address: SocketAddr,
    pub ipv4_type: bool,
    pub domain: String,
    pub cert: rustls::Certificate,
    /// should be set only on client.
    pub keep_alive_interval: Duration,
    pub max_bi_streams: usize,
}

pub struct ClientConfigBuilder {
    #[allow(unused)]
    pub remote_address: Option<SocketAddr>,
    #[allow(unused)]
    pub ipv4_type: Option<bool>,
    #[allow(unused)]
    pub domain: Option<String>,
    #[allow(unused)]
    pub cert: Option<rustls::Certificate>,
    #[allow(unused)]
    pub keep_alive_interval: Option<Duration>,
    #[allow(unused)]
    pub max_bi_streams: Option<usize>,
}

impl Default for ClientConfigBuilder {
    fn default() -> Self {
        Self {
            remote_address: None,
            ipv4_type: None,
            domain: None,
            cert: None,
            keep_alive_interval: None,
            max_bi_streams: None,
        }
    }
}

impl ClientConfigBuilder {
    pub fn with_remote_address(&mut self, remote_address: SocketAddr) -> &mut Self {
        self.remote_address = Some(remote_address);
        self
    }

    pub fn with_ipv4_type(&mut self, ipv4_type: bool) -> &mut Self {
        self.ipv4_type = Some(ipv4_type);
        self
    }

    pub fn with_domain(&mut self, domain: String) -> &mut Self {
        self.domain = Some(domain);
        self
    }

    pub fn with_cert(&mut self, cert: rustls::Certificate) -> &mut Self {
        self.cert = Some(cert);
        self
    }

    pub fn with_keep_alive_interval(&mut self, keep_alive_interval: Duration) -> &mut Self {
        self.keep_alive_interval = Some(keep_alive_interval);
        self
    }

    pub fn with_max_bi_streams(&mut self, max_bi_streams: usize) -> &mut Self {
        self.max_bi_streams = Some(max_bi_streams);
        self
    }

    pub fn build(self) -> Result<ClientConfig> {
        let remote_address = self
            .remote_address
            .ok_or_else(|| anyhow!("address is required"))?;
        let ipv4_type = self
            .ipv4_type
            .ok_or_else(|| anyhow!("ipv4_type is required"))?;
        let domain = self.domain.ok_or_else(|| anyhow!("domain is required"))?;
        let cert = self.cert.ok_or_else(|| anyhow!("cert is required"))?;
        let keep_alive_interval = self
            .keep_alive_interval
            .ok_or_else(|| anyhow!("keep_alive_interval is required"))?;
        let max_bi_streams = self
            .max_bi_streams
            .ok_or_else(|| anyhow!("max_bi_streams is required"))?;
        Ok(ClientConfig {
            remote_address,
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
        })
    }
}

/// client with no ack promise.
pub struct Client {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
    io_channel: Option<(MsgMpmcSender, MsgMpscReceiver)>,
    bridge_channel: Option<(MsgMpscSender, MsgMpmcReceiver)>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config: Some(config),
            endpoint: None,
            connection: None,
            io_channel: None,
            bridge_channel: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let ClientConfig {
            remote_address,
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
        } = self.config.take().unwrap();
        let default_address = if ipv4_type {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = Endpoint::client(default_address)?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut client_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        let (bridge_sender, io_receiver) = tokio::sync::mpsc::channel(64);
        let (io_sender, bridge_receiver) = async_channel::bounded(64);
        self.endpoint = Some(endpoint);
        self.connection = Some(connection);
        self.bridge_channel = Some((bridge_sender, bridge_receiver));
        self.io_channel = Some((io_sender, io_receiver));
        Ok(())
    }

    #[allow(unused)]
    pub async fn new_net_streams(
        &mut self,
        // every new stream needed to be authenticated.
        auth_msg: Arc<Msg>,
    ) -> Result<quinn::StreamId> {
        let mut io_streams = self.connection.as_ref().unwrap().open_bi().await?;
        let stream_id = io_streams.0.id();
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let bridge_channel = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let mut io_operators = MsgIOWrapper::new(io_streams.0, io_streams.1);
        let (send_channel, mut recv_channel) = io_operators.channels();
        if send_channel.send(auth_msg).await.is_err() {
            return Err(anyhow!("send auth msg failed"));
        }
        tokio::spawn(async move {
            loop {
                select! {
                    msg = recv_channel.recv() => {
                        match msg {
                            Some(msg) => {
                                if bridge_channel.0.send(msg).await.is_err() {
                                    break;
                                }
                            },
                            None => {
                                break;
                            },
                        }
                    },
                    msg = bridge_channel.1.recv() => {
                        match msg {
                            Ok(msg) => {
                                if send_channel.send(msg).await.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    }
                }
            }
        });
        Ok(stream_id)
    }

    #[allow(unused)]
    pub async fn io_channel_token(
        &mut self,
        sender: u64,
        receiver: u64,
        node_id: u32,
        token: &str,
    ) -> Result<(MsgMpmcSender, MsgMpscReceiver)> {
        let auth = Msg::auth(sender, receiver, node_id, token);
        self.new_net_streams(Arc::new(auth)).await?;
        let channel = self.io_channel().await?;
        Ok((channel.0, channel.1))
    }

    #[allow(unused)]
    pub async fn io_channel_server_info(
        &mut self,
        server_info: &ServerInfo,
    ) -> Result<(MsgMpmcSender, MsgMpscReceiver)> {
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        self.new_net_streams(Arc::new(auth)).await?;
        let channel = self.io_channel().await?;
        Ok((channel.0, channel.1))
    }

    #[allow(unused)]
    pub async fn io_channel(&mut self) -> Result<(MsgMpmcSender, MsgMpscReceiver)> {
        let mut channel = self.io_channel.take().unwrap();
        Ok(channel)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(endpoint) = self.endpoint.as_ref() {
            endpoint.close(0u32.into(), b"it's time to say goodbye.");
        }
    }
}

/// client with async timeout notification pattern.
pub struct ClientTimeout {
    pub(self) config: Option<ClientConfig>,
    pub(self) endpoint: Option<Endpoint>,
    pub(self) connection: Option<Connection>,
    /// providing operations for outer caller to interact with the underlayer io.
    pub(self) io_channel: Option<(MsgMpmcSender, MsgMpscReceiver)>,
    pub(self) bridge_channel: Option<(MsgMpscSender, MsgMpmcReceiver)>,
    pub(self) timeout_sender: Option<MsgMpscSender>,
    pub(self) timeout_receiver: Option<MsgMpscReceiver>,
    pub(self) timeout: Duration,
}

impl ClientTimeout {
    pub fn new(config: ClientConfig, timeout: Duration) -> Self {
        let (bridge_sender, io_receiver) = tokio::sync::mpsc::channel(64);
        let (io_sender, bridge_receiver) = async_channel::bounded(64);
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        Self {
            config: Some(config),
            endpoint: None,
            connection: None,
            io_channel: Some((io_sender, io_receiver)),
            bridge_channel: Some((bridge_sender, bridge_receiver)),
            timeout_sender: Some(timeout_sender),
            timeout_receiver: Some(timeout_receiver),
            timeout,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let ClientConfig {
            remote_address,
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
        } = self.config.take().unwrap();
        let default_address = if ipv4_type {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = Endpoint::client(default_address)?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut client_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        self.endpoint = Some(endpoint);
        self.connection = Some(connection);
        Ok(())
    }

    #[allow(unused)]
    pub async fn new_net_streams(&mut self, auth_msg: Arc<Msg>) -> Result<quinn::StreamId> {
        let mut io_streams = self.connection.as_ref().unwrap().open_bi().await?;
        let stream_id = io_streams.0.id();
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let (bridge_sender, bridge_receiver) = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let timeout_sender = self.timeout_sender.as_ref().unwrap().clone();
        let mut io_operators =
            MsgIOTimeoutWrapper::new(io_streams.0, io_streams.1, self.timeout, None);
        let (send_channel, mut recv_channel, mut timeout_channel) = io_operators.channels();
        if send_channel.send(auth_msg).await.is_err() {
            return Err(anyhow!("send auth msg failed"));
        }
        tokio::spawn(async move {
            loop {
                select! {
                    msg = recv_channel.recv() => {
                        match msg {
                            Some(msg) => {
                                let res = bridge_sender.send(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            None => {
                                break;
                            },
                        }
                    },
                    msg = bridge_receiver.recv() => {
                        match msg {
                            Ok(msg) => {
                                let res = send_channel.send(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    },
                    msg = timeout_channel.recv() => {
                        match msg {
                            Some(msg) => {
                                let res = timeout_sender.send(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            None => {
                                break;
                            },
                        }
                    },
                }
            }
        });
        Ok(stream_id)
    }

    #[allow(unused)]
    pub async fn io_channel_token(
        &mut self,
        sender: u64,
        receiver: u64,
        node_id: u32,
        token: &str,
    ) -> Result<(MsgMpmcSender, MsgMpscReceiver, MsgMpscReceiver)> {
        let auth = Msg::auth(sender, receiver, node_id, token);
        self.new_net_streams(Arc::new(auth)).await?;
        let channel = self.io_channel().await?;
        Ok((channel.0, channel.1, channel.2))
    }

    #[allow(unused)]
    pub async fn io_channel_server_info(
        &mut self,
        server_info: &ServerInfo,
        receiver: u64,
    ) -> Result<(MsgMpmcSender, MsgMpscReceiver, MsgMpscReceiver)> {
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        self.new_net_streams(Arc::new(auth)).await?;
        let channel = self.io_channel().await?;
        Ok((channel.0, channel.1, channel.2))
    }

    #[allow(unused)]
    pub async fn io_channel(
        &mut self,
    ) -> Result<(MsgMpmcSender, MsgMpscReceiver, MsgMpscReceiver)> {
        let mut channel = self.io_channel.take().unwrap();
        let timeout_receiver = self.timeout_receiver.take().unwrap();
        Ok((channel.0, channel.1, timeout_receiver))
    }
}

impl Drop for ClientTimeout {
    fn drop(&mut self) {
        if let Some(endpoint) = self.endpoint.as_ref() {
            endpoint.close(0u32.into(), b"it's time to say goodbye.");
        }
    }
}

/// client with multi connection by one endpoint.
/// may be useful on scene that to large client connection is required.
pub struct ClientMultiConnection {
    endpoint: Endpoint,
}

impl ClientMultiConnection {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let ClientConfig {
            ipv4_type,
            cert,
            keep_alive_interval,
            max_bi_streams,
            ..
        } = config;
        let default_address = if ipv4_type {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = Endpoint::client(default_address)?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut client_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        Ok(Self { endpoint })
    }

    pub async fn new_connection(
        &self,
        config: SubConnectionConfig,
        auth_msg: Arc<Msg>,
    ) -> Result<SubConnection> {
        let SubConnectionConfig {
            remote_address,
            domain,
            opened_bi_streams_number,
            ..
        } = config;
        let new_connection = self
            .endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        let (bridge_sender, io_receiver) = tokio::sync::mpsc::channel(64);
        let (io_sender, bridge_receiver) = async_channel::bounded(64);
        for _ in 0..opened_bi_streams_number {
            let io_streams = connection.open_bi().await?;
            let bridge_channel = (bridge_sender.clone(), bridge_receiver.clone());
            let mut io_operators = MsgIOWrapper::new(io_streams.0, io_streams.1);
            let (send_channel, mut recv_channel) = io_operators.channels();
            if send_channel.send(auth_msg.clone()).await.is_err() {
                return Err(anyhow!("send auth msg failed"));
            }
            tokio::spawn(async move {
                loop {
                    select! {
                        msg = recv_channel.recv() => {
                            match msg {
                                Some(msg) => {
                                    if bridge_channel.0.send(msg).await.is_err() {
                                        break;
                                    }
                                },
                                None => {
                                    break;
                                },
                            }
                        },
                        msg = bridge_channel.1.recv() => {
                            match msg {
                                Ok(msg) => {
                                    if send_channel.send(msg).await.is_err() {
                                        break;
                                    }
                                },
                                Err(_) => {
                                    break;
                                },
                            }
                        }
                    }
                }
            });
        }
        // we not implement uni stream
        Ok(SubConnection {
            connection,
            io_channel: Some((io_sender, io_receiver)),
        })
    }

    pub async fn new_timeout_connection(
        &self,
        config: SubConnectionConfig,
        auth_msg: Arc<Msg>,
    ) -> Result<SubConnectionTimeout> {
        let SubConnectionConfig {
            remote_address,
            domain,
            timeout,
            ..
        } = config;
        let new_connection = self
            .endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        let (bridge_sender, io_receiver) = tokio::sync::mpsc::channel(64);
        let (io_sender, bridge_receiver) = async_channel::bounded(64);
        let (timeout_channel_sender, timeout_channel_receiver) = tokio::sync::mpsc::channel(64);
        for _ in 0..config.opened_bi_streams_number {
            let mut io_streams = connection.open_bi().await?;
            let (bridge_sender, bridge_receiver) = (bridge_sender.clone(), bridge_receiver.clone());
            let timeout_channel_sender = timeout_channel_sender.clone();
            io_streams.0.write_all(auth_msg.as_slice()).await?;
            let mut io_operators =
                MsgIOTimeoutWrapper::new(io_streams.0, io_streams.1, timeout, None);
            let (send_channel, mut recv_channel, mut timeout_channel) = io_operators.channels();
            if send_channel.send(auth_msg.clone()).await.is_err() {
                return Err(anyhow!("send auth msg failed"));
            }
            tokio::spawn(async move {
                loop {
                    select! {
                        msg = recv_channel.recv() => {
                            match msg {
                                Some(msg) => {
                                    let res = bridge_sender.send(msg).await;
                                    if res.is_err() {
                                        break;
                                    }
                                },
                                None => {
                                    break;
                                },
                            }
                        },
                        msg = bridge_receiver.recv() => {
                            match msg {
                                Ok(msg) => {
                                    let res = send_channel.send(msg).await;
                                    if res.is_err() {
                                        break;
                                    }
                                },
                                Err(_) => {
                                    break;
                                },
                            }
                        },
                        msg = timeout_channel.recv() => {
                            match msg {
                                Some(msg) => {
                                    let res = timeout_channel_sender.send(msg).await;
                                    if res.is_err() {
                                        break;
                                    }
                                },
                                None => {
                                    break;
                                },
                            }
                        },
                    }
                }
            });
        }
        Ok(SubConnectionTimeout {
            connection,
            io_channel: Some((io_sender, io_receiver)),
            timeout_channel_receiver: Some(timeout_channel_receiver),
        })
    }
}

impl Drop for ClientMultiConnection {
    fn drop(&mut self) {
        self.endpoint
            .close(0u32.into(), b"it's time to say goodbye.");
    }
}

pub struct SubConnectionConfig {
    pub remote_address: SocketAddr,
    pub domain: String,
    pub opened_bi_streams_number: usize,
    pub timeout: Duration,
}

pub struct SubConnection {
    connection: Connection,
    io_channel: Option<(MsgMpmcSender, MsgMpscReceiver)>,
}

impl SubConnection {
    pub fn operation_channel(&mut self) -> (MsgMpmcSender, MsgMpscReceiver) {
        let (outer_sender, outer_receiver) = self.io_channel.take().unwrap();
        (outer_sender, outer_receiver)
    }
}

impl Drop for SubConnection {
    fn drop(&mut self) {
        self.connection
            .close(0u32.into(), b"it's time to say goodbye.");
    }
}

pub struct SubConnectionTimeout {
    connection: quinn::Connection,
    io_channel: Option<(MsgMpmcSender, MsgMpscReceiver)>,
    timeout_channel_receiver: Option<MsgMpscReceiver>,
}

impl SubConnectionTimeout {
    pub fn operation_channel(&mut self) -> (MsgMpmcSender, MsgMpscReceiver, MsgMpscReceiver) {
        let (outer_sender, outer_receiver) = self.io_channel.take().unwrap();
        (
            outer_sender,
            outer_receiver,
            self.timeout_channel_receiver.take().unwrap(),
        )
    }
}

impl Drop for SubConnectionTimeout {
    fn drop(&mut self) {
        self.connection
            .close(0u32.into(), b"it's time to say goodbye.");
    }
}

pub struct ClientTlsTimeout {
    config: Option<ClientConfig>,
    io_channel: Option<(MsgMpmcSender, MsgMpscReceiver)>,
    bridge_channel: Option<(MsgMpscSender, MsgMpmcReceiver)>,
    timeout_sender: Option<MsgMpscSender>,
    timeout_receiver: Option<MsgMpscReceiver>,
    connection: Option<TlsStream<TcpStream>>,
    timeout: Duration,
    keep_alive_interval: Duration,
}

impl ClientTlsTimeout {
    pub fn new(config: ClientConfig, timeout: Duration) -> Self {
        let (bridge_sender, io_receiver) = tokio::sync::mpsc::channel(64);
        let (io_sender, bridge_receiver) = async_channel::bounded(64);
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        let keep_live_interval = config.keep_alive_interval;
        ClientTlsTimeout {
            config: Some(config),
            io_channel: Some((io_sender, io_receiver)),
            bridge_channel: Some((bridge_sender, bridge_receiver)),
            timeout_sender: Some(timeout_sender),
            timeout_receiver: Some(timeout_receiver),
            connection: None,
            timeout,
            keep_alive_interval: keep_live_interval,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let ClientConfig {
            remote_address,
            domain,
            cert,
            ..
        } = self.config.take().unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let connector = TlsConnector::from(Arc::new(client_crypto));
        let stream = TcpStream::connect(remote_address).await?;
        let domain = rustls::ServerName::try_from(domain.as_str()).unwrap();
        let stream = connector.connect(domain, stream).await?;
        self.connection = Some(stream);
        Ok(())
    }

    pub async fn new_net_streams(&mut self, auth_msg: Arc<Msg>) -> Result<()> {
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let (bridge_sender, bridge_receiver) = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let timeout_channel_sender = self.timeout_sender.as_ref().unwrap().clone();
        let (reader, writer) = split(self.connection.take().unwrap());
        let mut io_operators = MsgIOTlsClientTimeoutWrapper::new(
            writer,
            reader,
            self.timeout,
            self.keep_alive_interval,
            None,
        );
        let (send_channel, mut recv_channel, mut timeout_channel) = io_operators.channels();
        if send_channel.send(auth_msg).await.is_err() {
            return Err(anyhow!("send auth msg failed"));
        }
        tokio::spawn(async move {
            loop {
                select! {
                    msg = recv_channel.recv() => {
                        match msg {
                            Some(msg) => {
                                if bridge_sender.send(msg).await.is_err() {
                                    break;
                                }
                            },
                            None => {
                                break;
                            },
                        }
                    },
                    msg = bridge_receiver.recv() => {
                        match msg {
                            Ok(msg) => {
                                if send_channel.send(msg).await.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    },
                    msg = timeout_channel.recv() => {
                        match msg {
                            Some(msg) => {
                                if timeout_channel_sender.send(msg).await.is_err() {
                                    break;
                                }
                            },
                            None => {
                                break;
                            },
                        }
                    },
                }
            }
        });
        Ok(())
    }

    pub async fn io_channel_token(
        &mut self,
        sender: u64,
        receiver: u64,
        node_id: u32,
        token: &str,
    ) -> Result<(MsgMpmcSender, MsgMpscReceiver, MsgMpscReceiver)> {
        let auth = Msg::auth(sender, receiver, node_id, token);
        self.new_net_streams(Arc::new(auth)).await?;
        let io_channel = self.io_channel.take().unwrap();
        let timeout_receiver = self.timeout_receiver.take().unwrap();
        Ok((io_channel.0, io_channel.1, timeout_receiver))
    }
}

pub(self) struct Operator(
    AtomicU64,
    tokio::sync::mpsc::Sender<(
        u64,
        ReqwestMsg,
        tokio::sync::oneshot::Sender<ReqwestMsg>,
        Waker,
    )>,
    u16,
);

impl std::cmp::PartialEq for Operator {
    fn eq(&self, other: &Self) -> bool {
        self.2 == other.2
    }
}

impl std::cmp::Eq for Operator {}

impl std::hash::Hash for Operator {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.2.hash(state);
    }
}

pub struct ClientReqwest {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
    operator_list: Vec<Operator>,
    remaining_streams: Arc<AtomicU16>,
    count: AtomicUsize,
}

impl ClientReqwest {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config: Some(config),
            endpoint: None,
            connection: None,
            operator_list: Vec::new(),
            remaining_streams: Arc::new(AtomicU16::new(0)),
            count: AtomicUsize::new(0),
        }
    }

    pub async fn build(&mut self) -> Result<()> {
        let ClientConfig {
            remote_address,
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
        } = self.config.take().unwrap();
        let default_address = if ipv4_type {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = Endpoint::client(default_address)?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut client_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        for i in 0..max_bi_streams {
            let req_id = AtomicU64::new(0);
            let (sender, mut receiver) = tokio::sync::mpsc::channel::<(
                u64,
                ReqwestMsg,
                tokio::sync::oneshot::Sender<ReqwestMsg>,
                Waker,
            )>(1024);
            let (mut send_stream, mut recv_stream) = match connection.open_bi().await {
                Ok(v) => v,
                Err(e) => {
                    error!("open streams error: {}", e.to_string());
                    continue;
                }
            };
            tokio::spawn(async move {
                let mut resp_sender_map = AHashMap::new();
                let mut waker_map = AHashMap::new();
                loop {
                    select! {
                        req = receiver.recv() => {
                            match req {
                                Some((req_id, req, sender, waker)) => {
                                    resp_sender_map.insert(req_id, sender);
                                    waker_map.insert(req_id, waker);
                                    if let Err(e) = ReqwestMsgIOUtil::send_msg(&req, &mut send_stream, None).await {
                                        error!("send msg error: {}", e.to_string());
                                        break;
                                    }
                                },
                                None => {
                                    error!("receiver closed.");
                                    break;
                                }
                            }
                        },
                        resp = ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None) => {
                            match resp {
                                Ok(resp) => {
                                    let req_id = resp.req_id();
                                    let sender = resp_sender_map.remove(&req_id);
                                    if sender.is_none() {
                                        error!("req_id: {} not found.", req_id);
                                        continue;
                                    }
                                    let sender = sender.unwrap();
                                    _ = sender.send(resp);
                                    match waker_map.remove(&req_id) {
                                        Some(waker) => {
                                            waker.wake();
                                        },
                                        None => {
                                            error!("req_id: {} not found.", req_id)
                                        }
                                    }
                                },
                                Err(e) => {
                                    error!("recv msg error: {}", e.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
            });
            self.operator_list.push(Operator(req_id, sender, i as u16));
        }
        self.endpoint = Some(endpoint);
        self.connection = Some(connection);
        self.remaining_streams = Arc::new(AtomicU16::new(max_bi_streams as u16));
        Ok(())
    }

    pub fn call(&self, mut req: ReqwestMsg) -> Result<Reqwest> {
        let index = self.count.fetch_add(1, Ordering::SeqCst);
        let operator = &self.operator_list[index % self.operator_list.len()];
        let req_id = operator.0.fetch_add(1, Ordering::SeqCst);
        let req_sender = operator.1.clone();
        req.set_req_id(req_id);
        // println!("count: {}", index);
        Ok(Reqwest {
            req_id,
            req: Some(req),
            sender_task: None,
            resp_receiver: None,
            sender_task_done: false,
            operator_sender: Some(req_sender),
        })
    }
}

impl Drop for ClientReqwest {
    fn drop(&mut self) {
        self.connection
            .as_ref()
            .unwrap()
            .close(0u32.into(), b"it's time to say goodbye.");
    }
}

/// the request will not sent until the future is polled.
pub struct Reqwest<'a> {
    req_id: u64,
    sender_task_done: bool,
    req: Option<ReqwestMsg>,
    operator_sender: Option<
        tokio::sync::mpsc::Sender<(
            u64,
            ReqwestMsg,
            tokio::sync::oneshot::Sender<ReqwestMsg>,
            Waker,
        )>,
    >,
    sender_task: Option<BoxFuture<'a, Result<()>>>,
    resp_receiver: Option<tokio::sync::oneshot::Receiver<ReqwestMsg>>,
}

impl<'a> Future for Reqwest<'a> {
    type Output = Result<ReqwestMsg>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.sender_task_done {
            match self.sender_task.as_mut() {
                Some(task) => {
                    match task.as_mut().poll(cx) {
                        std::task::Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        std::task::Poll::Pending => {
                            return std::task::Poll::Pending;
                        }
                    };
                }
                None => {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let req = self.req.take().unwrap();
                    let req_id = self.req_id;
                    let waker = cx.waker().clone();
                    let operator_sender = self.operator_sender.take().unwrap();
                    let task = async move {
                        if let Err(e) = operator_sender.send((req_id, req, tx, waker)).await {
                            error!("send req error: {}", e.to_string());
                            return Err(anyhow!(e));
                        }
                        Ok(())
                    };
                    let task: BoxFuture<Result<()>> = Box::pin(task);
                    self.sender_task = Some(task);
                    self.resp_receiver = Some(rx);
                    match self.sender_task.as_mut().unwrap().as_mut().poll(cx) {
                        std::task::Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        std::task::Poll::Pending => {
                            return std::task::Poll::Pending;
                        }
                    };
                }
            };
        }
        match self.resp_receiver.as_mut().unwrap().try_recv() {
            Ok(resp) => std::task::Poll::Ready(Ok(resp)),
            Err(_) => std::task::Poll::Pending,
        }
    }
}
