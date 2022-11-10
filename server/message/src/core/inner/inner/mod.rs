mod client;
mod handler;
mod server;

use common::entity::{ServerInfo, ServerType, Type};
use common::net::OuterReceiver;
use common::Result;

use common::net::server::{HandlerList, NewConnectionHandlerGenerator, ServerConfigBuilder, Server};
use common::util::is_ipv6_enabled;
use std::sync::Arc;
use tracing::{error, warn};

use crate::config::CONFIG;
use crate::core::get_cluster_connection_map;
use crate::core::outer::handler::message::Text;
use crate::util::should_send_to_peer;

use self::server::ClusterConnectionHandler;

pub(self) async fn io_tasks(_outer_receiver: OuterReceiver) -> Result<()> {
    Ok(())
}

pub(crate) async fn start(mut msg_receiver: OuterReceiver) -> Result<()> {
    let (io_sender, io_receiver) =
        tokio::sync::mpsc::channel(CONFIG.performance.max_receiver_side_channel_size);
    let mut handler_list: HandlerList = Arc::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Text {}));
    let handler_list_clone = handler_list.clone();
    let io_sender_clone = io_sender.clone();
    let new_connection_handler_generator: NewConnectionHandlerGenerator = Box::new(move || {
        Box::new(ClusterConnectionHandler::new(
            handler_list_clone.clone(),
            io_sender_clone.clone(),
        ))
    });
    let address = CONFIG.server.inner_address;
    if address.is_ipv6() && !is_ipv6_enabled() {
        panic!("ipv6 is not enabled on this machine");
    }
    let mut server_config_builder = ServerConfigBuilder::default();
    server_config_builder
        .with_address(CONFIG.server.inner_address)
        .with_cert(CONFIG.server.cert.clone())
        .with_key(CONFIG.server.key.clone())
        .with_max_connections(CONFIG.server.max_connections)
        .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams)
        .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
        .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
    let server_config = server_config_builder.build();
    let mut server = Server::new(server_config.unwrap());
    tokio::spawn(async move {
        let res = io_tasks(io_receiver).await;
        if let Err(e) = res {
            error!("io_tasks error: {}", e);
        }
    });
    tokio::spawn(async move {
        let res = server.run(new_connection_handler_generator).await;
        if let Err(e) = res {
            error!("server run error: {}", e);
        }
    });
    let cluster_map = get_cluster_connection_map();
    let client = client::Client::new(io_sender, handler_list.clone()).await?;
    loop {
        let msg = msg_receiver.recv().await;
        if msg.is_none() {
            error!("channel closed");
        }
        let msg = msg.unwrap();
        match msg.typ() {
            Type::NodeRegister => {
                let server_info = ServerInfo::from(msg.payload());
                match server_info.typ {
                    ServerType::MessageCluster => {
                        let new_peer = msg.extension()[0] == 1;
                        if should_send_to_peer(server_info.id, new_peer) {
                            client.run(&server_info).await?;
                        }
                    }
                    _ => {}
                }
            }
            Type::NodeUnregister => {
                let server_info = ServerInfo::from(msg.payload());
                match server_info.typ {
                    ServerType::MessageCluster => {
                        error!("peer[{}] dead", server_info.id);
                        cluster_map.remove(&server_info.id);
                    }
                    _ => {}
                }
            }
            _ => {
                warn!("unknown message type: {}", msg.typ());
                continue;
            }
        };
    }
}
