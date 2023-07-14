use std::sync::{atomic::AtomicUsize, Arc};

use anyhow::anyhow;
use dashmap::{mapref::one::Ref, DashMap};
use lazy_static::lazy_static;
use lib::{net::GenericParameter, Result};
use lib_net_tokio::net::{client::ClientReqwestTcp, MsgSender};
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, ResourceSpecifier, TopicReplication},
    client::DefaultClientContext,
    ClientConfig,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::error;

use self::handler::io_task;
use crate::{config::CONFIG, service::handler::IOTaskReceiver, util::my_id, CPU_NUM};

pub(crate) mod handler;
pub(crate) mod server;

pub(crate) struct ClientConnectionMap(pub(crate) Arc<DashMap<u64, MsgSender>>);
pub(crate) struct MsgloggerClient(pub(crate) tokio::net::UnixStream);

lazy_static! {
    pub(self) static ref CLIENT_CONNECTION_MAP: ClientConnectionMap =
        ClientConnectionMap(Arc::new(DashMap::new()));
    pub(self) static ref SEQNUM_CLIENT_HOLDER: Arc<DashMap<u32, ClientReqwestTcp>> =
        Arc::new(DashMap::new());
    pub(self) static ref CLIENT_INDEX: AtomicUsize = AtomicUsize::new(0);
}

pub(crate) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CLIENT_CONNECTION_MAP.0.clone())
}

pub(crate) fn get_seqnum_client_holder() -> Arc<DashMap<u32, ClientReqwestTcp>> {
    SEQNUM_CLIENT_HOLDER.clone()
}

pub(crate) async fn get_msglogger_client() -> Result<MsgloggerClient> {
    let index = CLIENT_INDEX.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let index = index % unsafe { CPU_NUM };
    let addr = format!("/tmp/msglogger-{}.sock", index);
    let stream = tokio::net::UnixStream::connect(addr).await?;
    Ok(MsgloggerClient(stream))
}

impl GenericParameter for ClientConnectionMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for MsgloggerClient {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ClientConnectionMap {
    pub(crate) fn get<'a>(&'a self, id: &u64) -> Option<Ref<'a, u64, MsgSender>> {
        self.0.get(id)
    }

    pub(crate) fn insert(&self, id: u64, sender: MsgSender) {
        self.0.insert(id, sender);
    }
}

impl MsgloggerClient {
    pub(crate) async fn log(&mut self, msg: &[u8]) -> Result<()> {
        self.0.write_all(msg).await?;
        let a = self.0.read_u8().await?;
        let b = self.0.read_u8().await?;
        if a != b'o' || b != b'k' {
            error!("msglogger client log error");
            return Err(anyhow!("msglogger client log error"));
        }
        Ok(())
    }
}

pub(crate) async fn start(io_task_receiver: IOTaskReceiver) -> Result<()> {
    let topic_name = format!("msg-{:06}", my_id());
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", &CONFIG.message_queue.address);

    let admin_client: AdminClient<DefaultClientContext> = client_config.create().unwrap();
    let admin_options =
        AdminOptions::new().operation_timeout(Some(std::time::Duration::from_secs(5)));

    let topic_metadata = match admin_client
        .describe_configs(vec![&ResourceSpecifier::Topic(&topic_name)], &admin_options)
        .await
    {
        Ok(topic_metadata) => topic_metadata,
        Err(e) => {
            error!("describe topic error: {}", e);
            return Err(anyhow!("describe topic error: {}", e));
        }
    };
    match topic_metadata[0] {
        Ok(ref item) => {
            if item.entries.len() == 0 {
                let partition = CONFIG
                    .message_queue
                    .address
                    .split(',')
                    .collect::<Vec<_>>()
                    .len() as i32;
                admin_client
                    .create_topics(
                        [&NewTopic::new(
                            &topic_name,
                            partition,
                            TopicReplication::Fixed(1),
                        )],
                        &admin_options,
                    )
                    .await?;
            }
        }
        Err(e) => {
            error!("describe topic error: {}", e);
            return Err(anyhow!("describe topic error: {}", e));
        }
    }
    tokio::spawn(async move {
        if let Err(e) = io_task(io_task_receiver).await {
            error!("io task error: {}", e);
        }
    });
    server::Server::run().await?;
    Ok(())
}
