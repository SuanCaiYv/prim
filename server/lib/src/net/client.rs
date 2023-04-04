use std::sync::atomic::AtomicU16;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use crate::{
    entity::{Msg, ServerInfo, TinyMsg, Type},
    Result,
};

use anyhow::anyhow;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::{io::split, net::TcpStream, select};
use tokio_rustls::{client::TlsStream, TlsConnector};

use super::{
    MsgIOTimeoutWrapper, MsgIOTlsClientTimeoutWrapper, MsgIOWrapper, MsgMpmcReceiver,
    MsgMpmcSender, MsgMpscReceiver, MsgMpscSender, TinyMsgIOUtil, ALPN_PRIM,
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
        auth_msg: Arc<Msg>,
    ) -> Result<(quinn::StreamId, Arc<Msg>)> {
        let mut io_streams = self.connection.as_ref().unwrap().open_bi().await?;
        let stream_id = io_streams.0.id();
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let bridge_channel = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let mut io_operators = MsgIOWrapper::new(io_streams.0, io_streams.1);
        let (send_channel, mut recv_channel) = io_operators.channels();
        if send_channel.send(auth_msg).await.is_err() {
            return Err(anyhow!("send auth msg failed"));
        }
        let auth_resp = recv_channel.recv().await;
        if auth_resp.is_none() {
            return Err(anyhow!("recv auth resp failed"));
        }
        let auth_resp = auth_resp.unwrap();
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
        Ok((stream_id, auth_resp))
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
        let mut channel = self.io_channel.take().unwrap();
        let auth_resp = channel.1.recv().await;
        if auth_resp.is_none() || auth_resp.unwrap().typ() != Type::Auth {
            return Err(anyhow!("auth failed"));
        }
        Ok(channel)
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
        let mut channel = self.io_channel.take().unwrap();
        let auth_resp = channel.1.recv().await;
        if auth_resp.is_none() || auth_resp.unwrap().typ() != Type::Auth {
            return Err(anyhow!("auth failed"));
        }
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
    pub(self) timeout_channel_sender: Option<MsgMpscSender>,
    pub(self) timeout_channel_receiver: Option<MsgMpscReceiver>,
    pub(self) timeout: Duration,
    pub(self) auth_msg: Option<Msg>,
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
            timeout_channel_sender: Some(timeout_sender),
            timeout_channel_receiver: Some(timeout_receiver),
            timeout,
            auth_msg: None,
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
    pub async fn new_net_streams(&mut self) -> Result<quinn::StreamId> {
        let mut io_streams = self.connection.as_ref().unwrap().open_bi().await?;
        let stream_id = io_streams.0.id();
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let (bridge_sender, bridge_receiver) = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let timeout_channel_sender = self.timeout_channel_sender.as_ref().unwrap().clone();
        io_streams
            .0
            .write_all(self.auth_msg.as_ref().unwrap().as_slice())
            .await?;
        let mut io_operators =
            MsgIOTimeoutWrapper::new(io_streams.0, io_streams.1, self.timeout, None);
        let (send_channel, mut recv_channel, mut timeout_channel) = io_operators.channels();
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
        self.auth_msg = Some(auth);
        self.new_net_streams().await?;
        let mut channel = self.io_channel.take().unwrap();
        let timeout_channel_receiver = self.timeout_channel_receiver.take().unwrap();
        let auth_resp = channel.1.recv().await;
        if auth_resp.is_none() || auth_resp.unwrap().typ() != Type::Auth {
            return Err(anyhow!("auth failed"));
        }
        Ok((channel.0, channel.1, timeout_channel_receiver))
    }

    #[allow(unused)]
    pub async fn io_channel_server_info(
        &mut self,
        server_info: &ServerInfo,
        receiver: u64,
    ) -> Result<(MsgMpmcSender, MsgMpscReceiver, MsgMpscReceiver, Arc<Msg>)> {
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        auth.set_receiver(receiver);
        self.auth_msg = Some(auth);
        self.new_net_streams().await?;
        let mut channel = self.io_channel.take().unwrap();
        let timeout_channel_receiver = self.timeout_channel_receiver.take().unwrap();
        let auth_resp = channel.1.recv().await;
        if auth_resp.is_none() || auth_resp.as_ref().unwrap().typ() != Type::Auth {
            return Err(anyhow!("auth failed"));
        }
        Ok((
            channel.0,
            channel.1,
            timeout_channel_receiver,
            auth_resp.unwrap(),
        ))
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
        auth_msg: &Msg,
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
            let mut io_streams = connection.open_bi().await?;
            let bridge_channel = (bridge_sender.clone(), bridge_receiver.clone());
            io_streams.0.write_all(auth_msg.as_slice()).await?;
            let mut io_operators = MsgIOWrapper::new(io_streams.0, io_streams.1);
            let (send_channel, mut recv_channel) = io_operators.channels();
            let auth_resp = recv_channel.recv().await;
            if auth_resp.is_none() || auth_resp.unwrap().typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
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
        auth_msg: &Msg,
    ) -> Result<(SubConnectionTimeout, Arc<Msg>)> {
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
        let mut resp_msg = None;
        for _ in 0..config.opened_bi_streams_number {
            let mut io_streams = connection.open_bi().await?;
            let (bridge_sender, bridge_receiver) = (bridge_sender.clone(), bridge_receiver.clone());
            let timeout_channel_sender = timeout_channel_sender.clone();
            io_streams.0.write_all(auth_msg.as_slice()).await?;
            let mut io_operators =
                MsgIOTimeoutWrapper::new(io_streams.0, io_streams.1, timeout, None);
            let (send_channel, mut recv_channel, mut timeout_channel) = io_operators.channels();
            let auth_resp = recv_channel.recv().await;
            if auth_resp.is_none() || auth_resp.as_ref().unwrap().typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            resp_msg = Some(auth_resp.unwrap());
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
        Ok((
            SubConnectionTimeout {
                connection,
                io_channel: Some((io_sender, io_receiver)),
                timeout_channel_receiver: Some(timeout_channel_receiver),
            },
            resp_msg.unwrap(),
        ))
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
    timeout_channel_sender: Option<MsgMpscSender>,
    timeout_channel_receiver: Option<MsgMpscReceiver>,
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
            timeout_channel_sender: Some(timeout_sender),
            timeout_channel_receiver: Some(timeout_receiver),
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

    pub async fn new_net_streams(&mut self) -> Result<()> {
        let bridge_channel = self.bridge_channel.as_ref().unwrap();
        let (bridge_sender, bridge_receiver) = (bridge_channel.0.clone(), bridge_channel.1.clone());
        let timeout_channel_sender = self.timeout_channel_sender.as_ref().unwrap().clone();
        let (reader, writer) = split(self.connection.take().unwrap());
        let mut io_operators = MsgIOTlsClientTimeoutWrapper::new(
            writer,
            reader,
            self.timeout,
            self.keep_alive_interval,
            None,
        );
        let (send_channel, mut recv_channel, mut timeout_channel) = io_operators.channels();
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
        Ok(ReqResp {
            connection,
            io_pair_sender: self.io_pair_sender.clone(),
            io_pair_receiver: self.io_pair_receiver.clone(),
            opened_streams: Arc::new(AtomicU16::new(max_bi_streams as u16)),
        })
    }
}

impl Drop for ClientReqResp {
    fn drop(&mut self) {
        if let Some(endpoint) = self.endpoint.take() {
            endpoint.close(0u32.into(), b"work has done.");
        }
    }
}

#[derive(Clone)]
pub struct ReqResp {
    connection: Connection,
    io_pair_sender: async_channel::Sender<(SendStream, RecvStream)>,
    io_pair_receiver: async_channel::Receiver<(SendStream, RecvStream)>,
    opened_streams: Arc<AtomicU16>,
}

impl ReqResp {
    pub async fn call(&self, msg: &TinyMsg) -> Result<TinyMsg> {
        if let Ok(pair) = self.io_pair_receiver.try_recv() {
            let (mut send_stream, mut recv_stream) = pair;
            TinyMsgIOUtil::send_msg(msg, &mut send_stream).await?;
            let res = TinyMsgIOUtil::recv_msg(&mut recv_stream).await?;
            self.io_pair_sender.send((send_stream, recv_stream)).await?;
            Ok(res)
        } else {
            if (self
                .opened_streams
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst) as usize)
                > 0
            {
                let (mut send_stream, mut recv_stream) = self.connection.open_bi().await?;
                TinyMsgIOUtil::send_msg(msg, &mut send_stream).await?;
                let res = TinyMsgIOUtil::recv_msg(&mut recv_stream).await?;
                self.io_pair_sender.send((send_stream, recv_stream)).await?;
                Ok(res)
            } else {
                let (mut send_stream, mut recv_stream) = self.io_pair_receiver.recv().await?;
                TinyMsgIOUtil::send_msg(msg, &mut send_stream).await?;
                let res = TinyMsgIOUtil::recv_msg(&mut recv_stream).await?;
                Ok(res)
            }
        }
    }
}
