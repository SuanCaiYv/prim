use std::sync::{atomic::AtomicUsize, Arc};

use anyhow::anyhow;
use dashmap::{mapref::one::Ref, DashMap};
use lazy_static::lazy_static;
use lib::{entity::Msg, net::GenericParameter, Result};
use lib_net_tokio::net::{client::ClientReqwestTcp, MsgSender};
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, ResourceSpecifier, TopicReplication},
    client::DefaultClientContext,
    ClientConfig,
};
use sysinfo::SystemExt;
use tracing::error;

use self::{handler::io_task, msglogger::MsgloggerClient};
use crate::{config::CONFIG, service::handler::{IOTaskSender, IOTaskReceiver, IOTaskMsg}, util::my_id};

pub(crate) mod handler;
pub(self) mod msglogger;
pub(crate) mod server;

pub(crate) struct ClientConnectionMap(pub(crate) Arc<DashMap<u64, MsgSender>>);

pub(crate) struct Msglogger(pub(self) Arc<MsgloggerClient>);

pub(crate) static mut IO_TASK_SENDER: Option<IOTaskSender> = None;
pub(crate) static mut IO_TASK_RECEIVER: Option<IOTaskReceiver> = None;

lazy_static! {
    pub(self) static ref CLIENT_CONNECTION_MAP: ClientConnectionMap =
        ClientConnectionMap(Arc::new(DashMap::new()));
    pub(self) static ref SEQNUM_CLIENT_HOLDER: Arc<DashMap<u32, ClientReqwestTcp>> =
        Arc::new(DashMap::new());
    pub(self) static ref CLIENT_INDEX: AtomicUsize = AtomicUsize::new(0);
    pub(self) static ref MSGLOGGER_CLIENT_MAP: Arc<DashMap<usize, Arc<MsgloggerClient>>> =
        Arc::new(DashMap::new());
}

pub(crate) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CLIENT_CONNECTION_MAP.0.clone())
}

pub(crate) fn get_seqnum_client_holder() -> Arc<DashMap<u32, ClientReqwestTcp>> {
    SEQNUM_CLIENT_HOLDER.clone()
}

pub(crate) fn get_msglogger_client() -> Msglogger {
    let index = CLIENT_INDEX.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let index = index % MSGLOGGER_CLIENT_MAP.len();
    Msglogger(MSGLOGGER_CLIENT_MAP.get(&index).unwrap().clone())
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

pub(crate) async fn load_msglogger() -> Result<()> {
    // create msglogger client
    let sys = sysinfo::System::new_all();
    for i in 0..sys.cpus().len() {
        let address = format!("/tmp/msglogger-{}.sock", i);
        let client = MsgloggerClient::new(address).await?;
        MSGLOGGER_CLIENT_MAP.insert(i, Arc::new(client));
    }
    Ok(())
}

pub(crate) async fn start() -> Result<()> {
    // create topic
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

    // start io task
    tokio::spawn(async move {
        if let Err(e) = io_task(unsafe { IO_TASK_RECEIVER.take().unwrap() }).await {
            error!("io task error: {}", e);
        }
    });
    server::Server::run().await?;
    Ok(())
}
