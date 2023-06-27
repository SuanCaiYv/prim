use std::{net::SocketAddr, sync::Arc, task::Waker, time::Duration};

use crate::{
    net::{NewReqwestConnectionHandler0, ReqwestMsgIOUtil, ReqwestOperator, ResponsePlaceholder},
    Result,
};

use anyhow::anyhow;

use async_trait::async_trait;
use dashmap::DashMap;
use futures_util::{pin_mut, FutureExt};
use lib::{
    entity::{Msg, ReqwestMsg, ServerInfo, Type},
    net::{client::ClientConfig, ALPN_PRIM},
};
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::{io::split, net::TcpStream, select, sync::mpsc};
use tokio_rustls::{client::TlsStream, TlsConnector};
use tracing::{debug, error};

use super::{
    MsgIOTimeoutWrapper, MsgIOTlsClientTimeoutWrapper, MsgIOWrapper, MsgMpmcReceiver,
    MsgMpmcSender, MsgMpscReceiver, MsgMpscSender, ReqwestHandlerGenerator,
    ReqwestHandlerGenerator0, ReqwestOperatorManager,
};

/// client with no ack promise.
pub struct Client {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
    io_channel: Option<(MsgMpmcSender, MsgMpscReceiver)>,
    bridge_channel: Option<(MsgMpscSender, MsgMpmcReceiver)>,
    max_connections: u16,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        let max_connections = config.max_bi_streams as u16;
        Self {
            config: Some(config),
            endpoint: None,
            connection: None,
            io_channel: None,
            bridge_channel: None,
            max_connections,
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
    pub(self) async fn new_net_streams(
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
        let mut channel = self.io_channel().await?;
        let auth = Msg::auth(sender, receiver, node_id, token);
        for _ in 0..self.max_connections {
            self.new_net_streams(Arc::new(auth.clone())).await?;
        }
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

pub(self) struct ClientReqwest0 {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
}

impl ClientReqwest0 {
    pub(self) fn new(config: ClientConfig) -> Self {
        ClientReqwest0 {
            config: Some(config),
            endpoint: None,
            connection: None,
        }
    }

    pub(self) async fn build<'a>(
        &'a mut self,
        generator: ReqwestHandlerGenerator0,
        operator_list: &'a mut Vec<ReqwestOperator>,
    ) -> Result<()> {
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

        let mut handler = generator();
        for _ in 0..max_bi_streams {
            let streams = match connection.open_bi().await {
                Ok(v) => v,
                Err(e) => {
                    error!("open streams error: {}", e.to_string());
                    continue;
                }
            };
            let operator: Option<ReqwestOperator> =
                handler.handle(streams, None).await.map_err(|e| {
                    error!("handle error: {}", e.to_string());
                    e
                })?;
            operator_list.push(operator.unwrap());
        }
        self.endpoint = Some(endpoint);
        self.connection = Some(connection);
        Ok(())
    }
}

impl Drop for ClientReqwest0 {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            connection.close(0u32.into(), b"ok");
        }
    }
}

pub struct ClientReqwest {
    client: ClientReqwest0,
    timeout: Duration,
}

impl ClientReqwest {
    pub fn new(config: ClientConfig, timeout: Duration) -> Self {
        let client = ClientReqwest0::new(config);
        ClientReqwest { client, timeout }
    }

    pub async fn build(
        &mut self,
        generator: Arc<ReqwestHandlerGenerator>,
    ) -> Result<ReqwestOperatorManager> {
        struct Generator0 {
            timeout: Duration,
            generator: Arc<ReqwestHandlerGenerator>,
        }

        #[async_trait]
        impl NewReqwestConnectionHandler0 for Generator0 {
            async fn handle(
                &mut self,
                msg_streams: (SendStream, RecvStream),
                _: Option<Arc<ReqwestOperatorManager>>,
            ) -> Result<Option<ReqwestOperator>> {
                let (mut send_stream, mut recv_stream) = msg_streams;

                let (sender, mut receiver) = mpsc::channel::<(
                    ReqwestMsg,
                    Option<(u64, Arc<ResponsePlaceholder>, Waker)>,
                )>(16384);
                let (msg_sender_outer, msg_receiver_outer) = mpsc::channel(16384);
                let (msg_sender_inner, mut msg_receiver_inner) = mpsc::channel(16384);

                let resp_waker_map0 = Arc::new(DashMap::new());
                let (tx, mut rx) = mpsc::channel::<u64>(4096);
                let stream_id = recv_stream.id().0;
                let sender_clone = sender.clone();
                let timeout = self.timeout;

                tokio::spawn(async move {
                    let resp_waker_map = resp_waker_map0.clone();

                    let task1 = async {
                        loop {
                            match receiver.recv().await {
                                Some((req, external)) => match external {
                                    // a request from client
                                    Some((req_id, sender, waker)) => {
                                        resp_waker_map.insert(req_id, (waker, sender));
                                        let res =
                                            ReqwestMsgIOUtil::send_msg(&req, &mut send_stream)
                                                .await;
                                        let tx = tx.clone();
                                        tokio::spawn(async move {
                                            tokio::time::sleep(timeout).await;
                                            _ = tx.send(req_id).await;
                                        });
                                        if let Err(e) = res {
                                            error!("send msg error: {}", e.to_string());
                                            break;
                                        }
                                    }
                                    // a response from client
                                    None => {
                                        if let Err(e) =
                                            ReqwestMsgIOUtil::send_msg(&req, &mut send_stream).await
                                        {
                                            error!("send msg error: {}", e.to_string());
                                            break;
                                        }
                                    }
                                },
                                None => {
                                    debug!("receiver closed.");
                                    _ = send_stream.finish().await;
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let resp_waker_map = resp_waker_map0.clone();

                    let task2 = async {
                        loop {
                            match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
                                Ok(msg) => {
                                    let req_id = msg.req_id();
                                    // a request from server
                                    if req_id ^ 0xF000_0000_0000_0000 == 0 {
                                        _ = msg_sender_outer.send(msg).await;
                                    } else {
                                        // a response from server
                                        match resp_waker_map.remove(&req_id) {
                                            Some(waker) => {
                                                waker.1 .0.wake();
                                                _ = waker.1 .1.set(Ok(msg));
                                            }
                                            None => {
                                                error!("req_id: {} not found.", req_id)
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    _ = recv_stream.stop(0u32.into());
                                    debug!("recv msg error: {}", e.to_string());
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let waker_map = resp_waker_map0;

                    let task3 = async {
                        loop {
                            match rx.recv().await {
                                Some(timeout_id) => match waker_map.remove(&timeout_id) {
                                    Some(waker) => {
                                        waker.1 .0.wake();
                                        _ = waker.1 .1.set(Err(anyhow!(
                                            "{:02} timeout: {}",
                                            stream_id,
                                            timeout_id
                                        )));
                                    }
                                    None => {}
                                },
                                None => {
                                    debug!("rx closed.");
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let task4 = async {
                        loop {
                            match msg_receiver_inner.recv().await {
                                Some(msg) => {
                                    let res = sender_clone.send((msg, None)).await;
                                    if let Err(e) = res {
                                        error!("send msg error: {}", e.to_string());
                                        break;
                                    }
                                }
                                None => {
                                    debug!("msg_receiver_inner closed.");
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    pin_mut!(task1, task2, task3, task4);

                    loop {
                        futures::select! {
                            _ = task1 => {},
                            _ = task2 => {},
                            _ = task3 => {},
                            _ = task4 => {},
                            complete => {
                                break;
                            }
                        }
                    }
                });

                let mut handler = (self.generator)();
                tokio::spawn(async move {
                    handler
                        .handle((msg_sender_inner, msg_receiver_outer))
                        .await
                        .map_err(|e| {
                            error!("handler error: {}", e.to_string());
                            e
                        })?;
                    Result::<()>::Ok(())
                });
                Ok(Some(ReqwestOperator(stream_id as u16, sender)))
            }
        }

        let timeout = self.timeout;
        let generator0: ReqwestHandlerGenerator0 = Box::new(move || {
            Box::new(Generator0 {
                timeout,
                generator: generator.clone(),
            })
        });
        let mut operator_list = Vec::new();
        self.client.build(generator0, &mut operator_list).await?;
        Ok(ReqwestOperatorManager::new_directly(operator_list))
    }
}

pub(self) struct ClientReqwestSub0 {
    connection: Connection,
    max_bi_streams: u16,
}

impl Drop for ClientReqwestSub0 {
    fn drop(&mut self) {
        self.connection.close(0u32.into(), b"ok");
    }
}

impl ClientReqwestSub0 {
    pub(self) async fn build<'a>(
        &'a mut self,
        generator: Arc<ReqwestHandlerGenerator0>,
        operator_list: &'a mut Vec<ReqwestOperator>,
    ) -> Result<()> {
        let mut handler = generator();
        for _ in 0..self.max_bi_streams {
            let streams = match self.connection.open_bi().await {
                Ok(v) => v,
                Err(e) => {
                    error!("open streams error: {}", e.to_string());
                    continue;
                }
            };
            let operator: Option<ReqwestOperator> =
                handler.handle(streams, None).await.map_err(|e| {
                    error!("handle error: {}", e.to_string());
                    e
                })?;
            operator_list.push(operator.unwrap());
        }
        Ok(())
    }
}

pub(self) struct ClientReqwestShare0 {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    domain: String,
    max_bi_streams: usize,
}

impl ClientReqwestShare0 {
    pub(self) fn new(config: ClientConfig) -> Self {
        Self {
            config: Some(config),
            endpoint: None,
            domain: "".to_string(),
            max_bi_streams: 1,
        }
    }

    pub(self) async fn build(&mut self) -> Result<()> {
        let ClientConfig {
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
            ..
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
        self.endpoint = Some(endpoint);
        self.domain = domain;
        self.max_bi_streams = max_bi_streams;
        Ok(())
    }

    pub(self) async fn new_connection(
        &self,
        remote_address: SocketAddr,
    ) -> Result<ClientReqwestSub0> {
        let new_connection = self
            .endpoint
            .as_ref()
            .unwrap()
            .connect(remote_address, self.domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect with reason: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        Ok(ClientReqwestSub0 {
            connection,
            max_bi_streams: self.max_bi_streams as u16,
        })
    }
}

pub struct ClientReqwestSub {
    sub_conn: ClientReqwestSub0,
    generator0: Arc<ReqwestHandlerGenerator0>,
}

impl ClientReqwestSub {
    pub async fn build(&mut self) -> Result<ReqwestOperatorManager> {
        let mut operator_list = Vec::new();
        self.sub_conn
            .build(self.generator0.clone(), &mut operator_list)
            .await?;
        Ok(ReqwestOperatorManager::new_directly(operator_list))
    }
}

pub struct ClientReqwestShare {
    client: ClientReqwestShare0,
    timeout: Duration,
    generator0: Option<Arc<ReqwestHandlerGenerator0>>,
}

impl ClientReqwestShare {
    pub fn new(config: ClientConfig, timeout: Duration) -> Self {
        let client = ClientReqwestShare0::new(config);
        Self {
            client,
            timeout,
            generator0: None,
        }
    }

    pub async fn build(&mut self, generator: ReqwestHandlerGenerator) -> Result<()> {
        self.client.build().await?;

        struct Generator0 {
            timeout: Duration,
            generator: Arc<ReqwestHandlerGenerator>,
        }

        #[async_trait]
        impl NewReqwestConnectionHandler0 for Generator0 {
            async fn handle(
                &mut self,
                msg_streams: (SendStream, RecvStream),
                _: Option<Arc<ReqwestOperatorManager>>,
            ) -> Result<Option<ReqwestOperator>> {
                let (mut send_stream, mut recv_stream) = msg_streams;

                let (sender, mut receiver) = mpsc::channel::<(
                    ReqwestMsg,
                    Option<(u64, Arc<ResponsePlaceholder>, Waker)>,
                )>(16384);
                let (msg_sender_outer, msg_receiver_outer) = mpsc::channel(16384);
                let (msg_sender_inner, mut msg_receiver_inner) = mpsc::channel(16384);

                let resp_sender_map0 = Arc::new(DashMap::new());
                let waker_map0 = Arc::new(DashMap::new());
                let (tx, mut rx) = mpsc::channel::<u64>(4096);
                let stream_id = recv_stream.id().0;
                let sender_clone = sender.clone();
                let timeout = self.timeout;

                tokio::spawn(async move {
                    let resp_sender_map = resp_sender_map0.clone();
                    let waker_map = waker_map0.clone();

                    let task1 = async {
                        loop {
                            match receiver.recv().await {
                                Some((req, external)) => match external {
                                    // a request from client
                                    Some((req_id, sender, waker)) => {
                                        resp_sender_map.insert(req_id, sender);
                                        waker_map.insert(req_id, waker);
                                        let res =
                                            ReqwestMsgIOUtil::send_msg(&req, &mut send_stream)
                                                .await;
                                        let tx = tx.clone();
                                        tokio::spawn(async move {
                                            tokio::time::sleep(timeout).await;
                                            _ = tx.send(req_id).await;
                                        });
                                        if let Err(e) = res {
                                            error!("send msg error: {}", e.to_string());
                                            break;
                                        }
                                    }
                                    // a response from client
                                    None => {
                                        if let Err(e) =
                                            ReqwestMsgIOUtil::send_msg(&req, &mut send_stream).await
                                        {
                                            error!("send msg error: {}", e.to_string());
                                            break;
                                        }
                                    }
                                },
                                None => {
                                    debug!("receiver closed.");
                                    _ = send_stream.finish().await;
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let resp_sender_map = resp_sender_map0.clone();
                    let waker_map = waker_map0.clone();

                    let task2 = async {
                        loop {
                            match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
                                Ok(msg) => {
                                    let req_id = msg.req_id();
                                    // a request from server
                                    if req_id ^ 0xF000_0000_0000_0000 == 0 {
                                        _ = msg_sender_outer.send(msg).await;
                                    } else {
                                        // a response from server
                                        match resp_sender_map.remove(&req_id) {
                                            Some(sender) => {
                                                _ = sender.1.set(Ok(msg));
                                            }
                                            None => {
                                                error!("req_id: {} not found.", req_id)
                                            }
                                        }
                                        match waker_map.remove(&req_id) {
                                            Some(waker) => {
                                                waker.1.wake();
                                            }
                                            None => {
                                                error!("req_id: {} not found.", req_id)
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    _ = recv_stream.stop(0u32.into());
                                    debug!("recv msg error: {}", e.to_string());
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let resp_sender_map = resp_sender_map0;
                    let waker_map = waker_map0;

                    let task3 = async {
                        loop {
                            match rx.recv().await {
                                Some(timeout_id) => {
                                    match resp_sender_map.remove(&timeout_id) {
                                        Some(sender) => {
                                            _ = sender.1.set(Err(anyhow!(
                                                "{:02} timeout: {}",
                                                stream_id,
                                                timeout_id
                                            )));
                                        }
                                        None => {}
                                    }
                                    match waker_map.remove(&timeout_id) {
                                        Some(waker) => {
                                            waker.1.wake();
                                        }
                                        None => {}
                                    }
                                }
                                None => {
                                    debug!("rx closed.");
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let task4 = async {
                        loop {
                            match msg_receiver_inner.recv().await {
                                Some(msg) => {
                                    let res = sender_clone.send((msg, None)).await;
                                    if let Err(e) = res {
                                        error!("send msg error: {}", e.to_string());
                                        break;
                                    }
                                }
                                None => {
                                    debug!("msg_receiver_inner closed.");
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    pin_mut!(task1, task2, task3, task4);

                    loop {
                        futures::select! {
                            _ = task1 => {},
                            _ = task2 => {},
                            _ = task3 => {},
                            _ = task4 => {},
                            complete => {
                                break;
                            }
                        }
                    }
                });

                let mut handler = (self.generator)();
                tokio::spawn(async move {
                    handler
                        .handle((msg_sender_inner, msg_receiver_outer))
                        .await
                        .map_err(|e| {
                            error!("handler error: {}", e.to_string());
                            e
                        })?;
                    Result::<()>::Ok(())
                });
                Ok(Some(ReqwestOperator(stream_id as u16, sender)))
            }
        }

        let generator = Arc::new(generator);
        let timeout = self.timeout;
        let generator0: ReqwestHandlerGenerator0 = Box::new(move || {
            Box::new(Generator0 {
                timeout,
                generator: generator.clone(),
            })
        });
        self.generator0 = Some(Arc::new(generator0));
        Ok(())
    }

    pub async fn new_connection(&self, remote_address: SocketAddr) -> Result<ClientReqwestSub> {
        let sub_conn = self.client.new_connection(remote_address).await?;
        Ok(ClientReqwestSub {
            sub_conn,
            generator0: self.generator0.clone().unwrap(),
        })
    }
}