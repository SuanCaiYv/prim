use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use ahash::AHashMap;
use anyhow::anyhow;
use dashmap::{mapref::one::Ref, DashMap};
use lazy_static::lazy_static;
use lib::{
    entity::Msg,
    net::{client::ClientConfigBuilder, GenericParameter},
    Result,
};
use lib_net_tokio::net::{client::ClientReqwestTcp, MsgSender, ReqwestOperatorManager};
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, ResourceSpecifier, TopicReplication},
    client::DefaultClientContext,
    producer::FutureProducer,
    ClientConfig,
};
use sysinfo::SystemExt;
use tokio::sync::RwLock;
use tracing::error;

use self::{handler::io_task, msglogger::MsgloggerClient};
use crate::{
    config::config,
    rpc::get_rpc_client,
    service::handler::{IOTaskMsg, IOTaskReceiver, IOTaskSender},
    util::my_id,
};

pub(crate) mod handler;
pub(self) mod msglogger;
pub(crate) mod server;

pub(crate) struct ClientConnectionMap(pub(crate) Arc<DashMap<u64, MsgSender>>);
#[derive(Clone)]
pub(crate) struct Msglogger(pub(self) Arc<MsgloggerClient>);

pub(crate) static mut IO_TASK_SENDER: Option<IOTaskSender> = None;
pub(crate) static mut IO_TASK_RECEIVER: Option<IOTaskReceiver> = None;

lazy_static! {
    // this map has a lot of write and read operations, so use dashmap is better
    pub(self) static ref CLIENT_CONNECTION_MAP: ClientConnectionMap =
    // this map's write operation is very rare, so use rwlock is better that dashmap
        ClientConnectionMap(Arc::new(DashMap::new()));
    pub(self) static ref SEQNUM_MAP: Arc<RwLock<AHashMap<u32, ReqwestOperatorManager>>> =
        Arc::new(RwLock::new(AHashMap::new()));
    // same as above
    pub(self) static ref SEQNUM_CLIENT_HOLDER: Arc<RwLock<AHashMap<u32, ClientReqwestTcp>>> =
        Arc::new(RwLock::new(AHashMap::new()));
    pub(self) static ref CLIENT_INDEX: AtomicUsize = AtomicUsize::new(0);
    pub(self) static ref MQ_PRODUCER: FutureProducer = load_producer();
}

/// this map's write operation only happens on application startup
/// so it's safe to use unsafe
static mut MSGLOGGER_CLIENT_MAP: Option<AHashMap<usize, Msglogger>> = None;

pub(crate) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CLIENT_CONNECTION_MAP.0.clone())
}

pub(crate) fn get_seqnum_client_map() -> Arc<RwLock<AHashMap<u32, ReqwestOperatorManager>>> {
    SEQNUM_MAP.clone()
}

pub(crate) fn get_seqnum_client_holder() -> Arc<RwLock<AHashMap<u32, ClientReqwestTcp>>> {
    SEQNUM_CLIENT_HOLDER.clone()
}

pub(crate) fn get_msglogger_client() -> Msglogger {
    let index = CLIENT_INDEX.fetch_add(1, Ordering::Acquire);
    unsafe {
        let map = MSGLOGGER_CLIENT_MAP.as_ref().unwrap();
        let index = index % map.len();
        map.get(&index).unwrap().clone()
    }
}

pub(crate) fn get_mq_producer() -> FutureProducer {
    MQ_PRODUCER.clone()
}

pub(crate) fn get_io_task_sender() -> &'static IOTaskSender {
    unsafe {
        &IO_TASK_SENDER
            .as_ref()
            .expect("io task sender not initialized")
    }
}

impl GenericParameter for ClientConnectionMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for Msglogger {
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

impl Msglogger {
    pub(crate) async fn log(&mut self, msg: Arc<Msg>) -> Result<()> {
        self.0.call(msg).await
    }
}

pub(crate) fn load_io_task() {
    // todo size optimization
    let (io_task_sender, io_task_receiver) = tokio::sync::mpsc::channel::<IOTaskMsg>(65536);
    unsafe {
        IO_TASK_SENDER = Some(IOTaskSender(io_task_sender));
        IO_TASK_RECEIVER = Some(IOTaskReceiver(io_task_receiver))
    };
}

pub(crate) async fn load_seqnum_map() -> Result<()> {
    let list = get_rpc_client().await.call_seqnum_all_node().await.unwrap();
    let address_list = list
        .1
        .into_iter()
        .map(|x| x.parse::<SocketAddr>().unwrap())
        .collect::<Vec<SocketAddr>>();
    let node_id_list = list.0;
    let client_map = get_seqnum_client_map();
    let client_holder = get_seqnum_client_holder();
    let mut map = client_map.write().await;
    let mut holder = client_holder.write().await;
    for (i, address) in address_list.into_iter().enumerate() {
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(address)
            .with_ipv4_type(address.is_ipv4())
            .with_domain(config().server.domain.clone())
            .with_cert(config().seqnum.cert.clone())
            .with_keep_alive_interval(config().transport.keep_alive_interval)
            .with_max_bi_streams(config().transport.max_bi_streams);
        let client_config = client_config.build().unwrap();
        let mut client = ClientReqwestTcp::new(client_config, Duration::from_millis(3000));
        let operator_manager = client.build().await.unwrap();
        let node_id = node_id_list[i];
        holder.insert(node_id, client);
        map.insert(node_id, operator_manager);
    }
    Ok(())
}

pub(crate) async fn load_msglogger() -> Result<()> {
    // create msglogger client
    let mut map = AHashMap::new();
    let sys = sysinfo::System::new_all();
    for i in 0..sys.cpus().len() {
        let address = format!("/tmp/msglogger-{}.sock", i);
        let client = MsgloggerClient::new(address).await?;
        map.insert(i, Msglogger(Arc::new(client)));
    }
    unsafe {
        MSGLOGGER_CLIENT_MAP = Some(map);
    }
    Ok(())
}

pub(self) fn load_producer() -> FutureProducer {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config().message_queue.address)
        .set("message.timeout.ms", "3000")
        .create()
        .unwrap();
    producer
}

pub(crate) async fn start() -> Result<()> {
    // create topic
    let topic_name = format!("msg-{:06}", my_id());
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", &config().message_queue.address);
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
                let partition = config()
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

    // start io task
    tokio::spawn(async move {
        if let Err(e) = io_task(unsafe { IO_TASK_RECEIVER.take().unwrap() }).await {
            error!("io task error: {}", e);
        }
    });

    load_seqnum_map().await?;
    server::Server::run().await?;
    Ok(())
}
