use std::{net::SocketAddr, sync::Arc, time::Duration};

use crate::{
    entity::{Msg, ServerInfo, Type, HEAD_LEN},
    net::MsgIOUtil,
    Result,
};
use ahash::AHashSet;
use anyhow::anyhow;
use quinn::{Connection, Endpoint};
use tokio::{io::split, net::TcpStream, select};
use tokio_rustls::{client::TlsStream, TlsConnector};

use super::{
    InnerReceiver, InnerSender, MsgIO2TimeoutUtil, MsgIOTimeoutUtil, OuterReceiver, OuterSender,
    ALPN_PRIM,
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
    pub max_uni_streams: usize,
    pub max_sender_side_channel_size: usize,
    pub max_receiver_side_channel_size: usize,
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
    #[allow(unused)]
    pub max_uni_streams: Option<usize>,
    #[allow(unused)]
    pub max_sender_side_channel_size: Option<usize>,
    #[allow(unused)]
    pub max_receiver_side_channel_size: Option<usize>,
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
            max_uni_streams: None,
            max_sender_side_channel_size: None,
            max_receiver_side_channel_size: None,
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

    pub fn with_max_uni_streams(&mut self, max_uni_streams: usize) -> &mut Self {
        self.max_uni_streams = Some(max_uni_streams);
        self
    }

    pub fn with_max_sender_side_channel_size(
        &mut self,
        max_sender_side_channel_size: usize,
    ) -> &mut Self {
        self.max_sender_side_channel_size = Some(max_sender_side_channel_size);
        self
    }

    pub fn with_max_receiver_side_channel_size(
        &mut self,
        max_receiver_side_channel_size: usize,
    ) -> &mut Self {
        self.max_receiver_side_channel_size = Some(max_receiver_side_channel_size);
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
        let max_uni_streams = self
            .max_uni_streams
            .ok_or_else(|| anyhow!("max_uni_streams is required"))?;
        let max_sender_side_channel_size = self
            .max_sender_side_channel_size
            .ok_or_else(|| anyhow!("max_sender_side_channel_size is required"))?;
        let max_receiver_side_channel_size = self
            .max_receiver_side_channel_size
            .ok_or_else(|| anyhow!("max_receiver_side_channel_size is required"))?;
        Ok(ClientConfig {
            remote_address,
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
            max_uni_streams,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
        })
    }
}

/// client with no ack promise.
pub struct Client {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
    io_channel: Option<(OuterSender, OuterReceiver)>,
    bridge_channel: Option<(InnerSender, InnerReceiver)>,
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
            max_uni_streams,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
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
            .max_concurrent_uni_streams(quinn::VarInt::from_u64(max_uni_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) = async_channel::bounded(max_sender_side_channel_size);
        self.endpoint = Some(endpoint);
        self.connection = Some(connection);
        self.bridge_channel = Some((bridge_sender, bridge_receiver));
        self.io_channel = Some((io_sender, io_receiver));
        Ok(())
    }

    #[allow(unused)]
    pub async fn new_net_streams(&mut self) -> Result<quinn::StreamId> {
        let mut streams = self.connection.as_ref().unwrap().open_bi().await?;
        let stream_id = streams.0.id();
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let bridge_channel = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let id = streams.0.id();
        tokio::spawn(async move {
            let mut buffer: Box<[u8; HEAD_LEN]> = Box::new([0_u8; HEAD_LEN]);
            loop {
                select! {
                    msg = MsgIOUtil::recv_msg(&mut buffer, &mut streams.1) => {
                        match msg {
                            Ok(msg) => {
                                let res = bridge_channel.0.send(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    },
                    msg = bridge_channel.1.recv() => {
                        match msg {
                            Ok(msg) => {
                                let res = MsgIOUtil::send_msg(msg, &mut streams.0).await;
                                if res.is_err() {
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
    ) -> Result<(OuterSender, OuterReceiver)> {
        self.new_net_streams().await?;
        let mut channel = self.io_channel.take().unwrap();
        let auth = Msg::auth(sender, receiver, node_id, token);
        channel.0.send(Arc::new(auth)).await?;
        let res = channel.1.recv().await;
        match res {
            Some(res) => {
                if res.typ() == Type::Auth {
                    Ok(channel)
                } else {
                    Err(anyhow!("auth failed"))
                }
            }
            None => Err(anyhow!("auth failed")),
        }
    }

    #[allow(unused)]
    pub async fn io_channel_server_info(
        &mut self,
        server_info: &ServerInfo,
        receiver: u64,
    ) -> Result<(OuterSender, OuterReceiver)> {
        self.new_net_streams().await?;
        let mut channel = self.io_channel.take().unwrap();
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        auth.set_receiver(receiver);
        channel.0.send(Arc::new(auth)).await?;
        let res = channel.1.recv().await;
        match res {
            Some(res) => {
                if res.typ() == Type::Auth {
                    Ok(channel)
                } else {
                    Err(anyhow!("auth failed"))
                }
            }
            None => Err(anyhow!("auth failed")),
        }
    }

    #[allow(unused)]
    pub async fn io_channel(&mut self) -> Result<(OuterSender, OuterReceiver)> {
        self.new_net_streams().await?;
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
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
    /// providing operations for outer caller to interact with the underlayer io.
    io_channel: Option<(OuterSender, OuterReceiver)>,
    bridge_channel: Option<(InnerSender, InnerReceiver)>,
    timeout_channel_sender: Option<InnerSender>,
    timeout_channel_receiver: Option<OuterReceiver>,
    timeout: Duration,
    max_receiver_side_channel_size: usize,
    ack_needed: bool,
}

impl ClientTimeout {
    pub fn new(config: ClientConfig, timeout: Duration, ack_needed: bool) -> Self {
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(config.max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) =
            async_channel::bounded(config.max_sender_side_channel_size);
        let (timeout_sender, timeout_receiver) =
            tokio::sync::mpsc::channel(config.max_receiver_side_channel_size);
        let max_receiver_side_channel_size = config.max_receiver_side_channel_size;
        Self {
            config: Some(config),
            endpoint: None,
            connection: None,
            io_channel: Some((io_sender, io_receiver)),
            bridge_channel: Some((bridge_sender, bridge_receiver)),
            timeout_channel_sender: Some(timeout_sender),
            timeout_channel_receiver: Some(timeout_receiver),
            timeout,
            max_receiver_side_channel_size,
            ack_needed,
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
            max_uni_streams,
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
            .max_concurrent_uni_streams(quinn::VarInt::from_u64(max_uni_streams as u64).unwrap())
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
    pub async fn new_net_streams(&mut self) -> Result<quinn::StreamId> {
        let mut io_streams = self.connection.as_ref().unwrap().open_bi().await?;
        let stream_id = io_streams.0.id();
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let (bridge_sender, bridge_receiver) = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let timeout_channel_sender = self.timeout_channel_sender.as_ref().unwrap().clone();
        let id = io_streams.0.id();
        let mut msg_io_timeout = MsgIOTimeoutUtil::new(
            io_streams,
            self.timeout,
            self.max_receiver_side_channel_size,
            Some(AHashSet::from_iter(vec![Type::Ack, Type::Auth])),
            self.ack_needed,
        );
        let mut timeout_channel_receiver = msg_io_timeout.timeout_channel_receiver();
        tokio::spawn(async move {
            loop {
                select! {
                    msg = msg_io_timeout.recv_msg() => {
                        match msg {
                            Ok(msg) => {
                                let res = bridge_sender.send(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    },
                    msg = bridge_receiver.recv() => {
                        match msg {
                            Ok(msg) => {
                                let res = msg_io_timeout.send_msg(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    },
                    msg = timeout_channel_receiver.recv() => {
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
        Ok(stream_id)
    }

    #[allow(unused)]
    pub async fn io_channel_token(
        &mut self,
        sender: u64,
        receiver: u64,
        node_id: u32,
        token: &str,
    ) -> Result<(OuterSender, OuterReceiver, OuterReceiver)> {
        self.new_net_streams().await?;
        let mut io_channel = self.io_channel.take().unwrap();
        let timeout_channel_receiver = self.timeout_channel_receiver.take().unwrap();
        let auth = Msg::auth(sender, receiver, node_id, token);
        io_channel.0.send(Arc::new(auth)).await?;
        let res = io_channel.1.recv().await;
        match res {
            Some(res) => {
                if res.typ() == Type::Auth {
                    Ok((io_channel.0, io_channel.1, timeout_channel_receiver))
                } else {
                    Err(anyhow!("auth failed"))
                }
            }
            None => Err(anyhow!("auth failed")),
        }
    }

    #[allow(unused)]
    pub async fn io_channel_server_info(
        &mut self,
        server_info: &ServerInfo,
        receiver: u64,
    ) -> Result<(OuterSender, OuterReceiver, OuterReceiver)> {
        self.new_net_streams().await?;
        let mut channel = self.io_channel.take().unwrap();
        let timeout_channel_receiver = self.timeout_channel_receiver.take().unwrap();
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        auth.set_receiver(receiver);
        channel.0.send(Arc::new(auth)).await?;
        let res = channel.1.recv().await;
        match res {
            Some(res) => {
                if res.typ() == Type::Auth {
                    Ok((channel.0, channel.1, timeout_channel_receiver))
                } else {
                    Err(anyhow!("auth failed"))
                }
            }
            None => Err(anyhow!("auth failed")),
        }
    }

    #[allow(unused)]
    pub async fn io_channel(&mut self) -> Result<(OuterSender, OuterReceiver, OuterReceiver)> {
        self.new_net_streams().await?;
        let mut channel = self.io_channel.take().unwrap();
        let timeout_channel_receiver = self.timeout_channel_receiver.take().unwrap();
        Ok((channel.0, channel.1, timeout_channel_receiver))
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
    max_sender_side_channel_size: usize,
    max_receiver_side_channel_size: usize,
}

impl ClientMultiConnection {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let ClientConfig {
            ipv4_type,
            cert,
            keep_alive_interval,
            max_bi_streams,
            max_uni_streams,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
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
            .max_concurrent_uni_streams(quinn::VarInt::from_u64(max_uni_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        Ok(Self {
            endpoint,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
        })
    }

    pub async fn new_connection(&self, config: SubConnectionConfig) -> Result<SubConnection> {
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
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(self.max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) =
            async_channel::bounded(self.max_sender_side_channel_size);
        for _ in 0..opened_bi_streams_number {
            let mut io_streams = connection.open_bi().await?;
            let bridge_channel = (bridge_sender.clone(), bridge_receiver.clone());
            tokio::spawn(async move {
                let mut buffer: Box<[u8; HEAD_LEN]> = Box::new([0_u8; HEAD_LEN]);
                loop {
                    select! {
                        msg = MsgIOUtil::recv_msg(&mut buffer, &mut io_streams.1) => {
                            match msg {
                                Ok(msg) => {
                                    let res = bridge_channel.0.send(msg).await;
                                    if res.is_err() {
                                        break;
                                    }
                                },
                                Err(_) => {
                                    break;
                                },
                            }
                        },
                        msg = bridge_channel.1.recv() => {
                            match msg {
                                Ok(msg) => {
                                    let res = MsgIOUtil::send_msg(msg, &mut io_streams.0).await;
                                    if res.is_err() {
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
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(self.max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) =
            async_channel::bounded(self.max_sender_side_channel_size);
        let (timeout_channel_sender, timeout_channel_receiver) =
            tokio::sync::mpsc::channel(self.max_receiver_side_channel_size);
        for _ in 0..config.opened_bi_streams_number {
            let io_streams = connection.open_bi().await?;
            let (bridge_sender, bridge_receiver) = (bridge_sender.clone(), bridge_receiver.clone());
            let timeout_channel_sender = timeout_channel_sender.clone();
            let mut msg_io_timeout = MsgIOTimeoutUtil::new(
                io_streams,
                timeout,
                self.max_receiver_side_channel_size,
                Some(AHashSet::from_iter(vec![Type::Ack, Type::Auth])),
                false,
            );
            let mut timeout_channel_receiver = msg_io_timeout.timeout_channel_receiver();
            tokio::spawn(async move {
                loop {
                    select! {
                        msg = msg_io_timeout.recv_msg() => {
                            match msg {
                                Ok(msg) => {
                                    let res = bridge_sender.send(msg).await;
                                    if res.is_err() {
                                        break;
                                    }
                                },
                                Err(_) => {
                                    break;
                                },
                            }
                        },
                        msg = bridge_receiver.recv() => {
                            match msg {
                                Ok(msg) => {
                                    let res = msg_io_timeout.send_msg(msg).await;
                                    if res.is_err() {
                                        break;
                                    }
                                },
                                Err(_) => {
                                    break;
                                },
                            }
                        },
                        msg = timeout_channel_receiver.recv() => {
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
    pub opened_uni_streams_number: usize,
    pub timeout: Duration,
}

pub struct SubConnection {
    connection: Connection,
    io_channel: Option<(OuterSender, OuterReceiver)>,
}

impl SubConnection {
    pub fn operation_channel(&mut self) -> (OuterSender, OuterReceiver) {
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
    io_channel: Option<(OuterSender, OuterReceiver)>,
    timeout_channel_receiver: Option<OuterReceiver>,
}

impl SubConnectionTimeout {
    pub fn operation_channel(&mut self) -> (OuterSender, OuterReceiver, OuterReceiver) {
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

pub struct Client2Timeout {
    config: Option<ClientConfig>,
    io_channel: Option<(OuterSender, OuterReceiver)>,
    bridge_channel: Option<(InnerSender, InnerReceiver)>,
    timeout_channel_sender: Option<InnerSender>,
    timeout_channel_receiver: Option<OuterReceiver>,
    connection: Option<TlsStream<TcpStream>>,
    timeout: Duration,
    max_receiver_side_channel_size: usize,
    keep_live_interval: Duration,
}

impl Client2Timeout {
    pub fn new(config: ClientConfig, timeout: Duration) -> Self {
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(config.max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) =
            async_channel::bounded(config.max_sender_side_channel_size);
        let (timeout_sender, timeout_receiver) =
            tokio::sync::mpsc::channel(config.max_receiver_side_channel_size);
        let max_receiver_side_channel_size = config.max_receiver_side_channel_size;
        let keep_live_interval = config.keep_alive_interval;
        Client2Timeout {
            config: Some(config),
            io_channel: Some((io_sender, io_receiver)),
            bridge_channel: Some((bridge_sender, bridge_receiver)),
            timeout_channel_sender: Some(timeout_sender),
            timeout_channel_receiver: Some(timeout_receiver),
            connection: None,
            timeout,
            max_receiver_side_channel_size,
            keep_live_interval,
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

    pub async fn new_net_streams(&mut self) -> Result<()> {
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let (bridge_sender, bridge_receiver) = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let timeout_channel_sender = self.timeout_channel_sender.as_ref().unwrap().clone();
        let (reader, writer) = split(self.connection.take().unwrap());
        let mut msg_io_timeout = MsgIO2TimeoutUtil::new(
            (writer, reader),
            self.timeout,
            self.max_receiver_side_channel_size,
            Some(AHashSet::from_iter(vec![Type::Ack, Type::Auth, Type::Ping])),
            true,
        );
        let mut ticker = tokio::time::interval(self.keep_live_interval);
        let mut timeout_channel_receiver = msg_io_timeout.timeout_channel_receiver();
        tokio::spawn(async move {
            loop {
                select! {
                    msg = msg_io_timeout.recv_msg() => {
                        match msg {
                            Ok(msg) => {
                                let res = bridge_sender.send(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    },
                    msg = bridge_receiver.recv() => {
                        match msg {
                            Ok(msg) => {
                                let res = msg_io_timeout.send_msg(msg).await;
                                if res.is_err() {
                                    break;
                                }
                            },
                            Err(_) => {
                                break;
                            },
                        }
                    },
                    msg = timeout_channel_receiver.recv() => {
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
                    _ = ticker.tick() => {
                        let msg = Arc::new(Msg::ping(0, 0, 0));
                        let res = msg_io_timeout.send_msg(msg).await;
                        if res.is_err() {
                            break;
                        }
                    }
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
    ) -> Result<(OuterSender, OuterReceiver, OuterReceiver)> {
        self.new_net_streams().await?;
        let mut io_channel = self.io_channel.take().unwrap();
        let timeout_channel_receiver = self.timeout_channel_receiver.take().unwrap();
        let auth = Msg::auth(sender, receiver, node_id, token);
        io_channel.0.send(Arc::new(auth)).await?;
        let res = io_channel.1.recv().await;
        match res {
            Some(res) => {
                if res.typ() == Type::Auth {
                    Ok((io_channel.0, io_channel.1, timeout_channel_receiver))
                } else {
                    Err(anyhow!("auth failed"))
                }
            }
            None => Err(anyhow!("auth failed")),
        }
    }
}

pub struct ClientReqResp {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    io_pair_sender: async_channel::Sender<(SendStream, RecvStream)>,
    io_pair_receiver: async_channel::Receiver<(SendStream, RecvStream)>,
}

impl ClientReqResp {
    pub fn new(config: ClientConfig) -> Self {
        let (io_pair_sender, io_pair_receiver) = async_channel::bounded(config.max_bi_streams);
        Self {
            config: Some(config),
            endpoint: None,
            io_pair_sender: io_pair_sender,
            io_pair_receiver: io_pair_receiver,
        }
    }

    pub async fn run(&mut self) -> Result<ReqResp> {
        let ClientConfig {
            remote_address,
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
            max_uni_streams,
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
            .max_concurrent_uni_streams(quinn::VarInt::from_u64(max_uni_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        self.endpoint = Some(endpoint);
        Ok(ReqResp {
            connection,
            io_pair_sender: self.io_pair_sender.clone(),
            io_pair_receiver: self.io_pair_receiver.clone(),
        })
    }
}

#[derive(Clone)]
pub struct ReqResp {
    connection: Connection,
    io_pair_sender: async_channel::Sender<(SendStream, RecvStream)>,
    io_pair_receiver: async_channel::Receiver<(SendStream, RecvStream)>,
}

impl ReqResp {
    pub async fn call(&self, msg: &TinyMsg) -> Result<TinyMsg> {
        if let Ok(pair) = self.io_pair_receiver.try_recv() {
            let (mut send_stream, mut recv_stream) = pair;
            let res = send_stream.write_all(msg.0.as_slice()).await;
            if let Err(e) = res {
                send_stream.finish().await;
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "write stream error.".to_string()
                )));
            }
            let mut len_buffer = [0u8; 2];
            if let Err(e) = recv_stream.read_exact(&mut len_buffer[..]).await {
                return match e {
                    ReadExactError::FinishedEarly => {
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "stream finished.".to_string()
                        )))
                    }
                    ReadExactError::ReadError(e) => {
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "read stream error.".to_string()
                        )))
                    }
                };
            }
            Ok(TinyMsg::default())
        } else {
            if let Ok(pair) = self.connection.open_bi().await {
                Ok(TinyMsg::default())
            } else {
                let (send_stream, recv_stream) = self.io_pair_receiver.recv().await?;
                Ok(TinyMsg::default())
            }
        }
    }
}
