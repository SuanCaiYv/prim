pub(super) mod server;
pub(super) mod handler;

use std::sync::Arc;
use tracing::error;
use common::net::{InnerSender, OuterReceiver};
use common::net::server::{HandlerList, NewConnectionHandlerGenerator, Server, ServerConfigBuilder};
use common::Result;
use common::util::is_ipv6_enabled;
use crate::config::CONFIG;
use crate::core::outer::handler::io_tasks;
use crate::core::outer::handler::logic::Auth;
use crate::core::outer::server::ClientConnectionHandler;

use self::handler::business::{Relationship, Group};
use self::handler::logic::Echo;
use self::handler::message::Text;

pub(crate) async fn start() -> Result<()> {
    let outer_channel: (InnerSender, OuterReceiver) =
        tokio::sync::mpsc::channel(CONFIG.performance.max_sender_side_channel_size);
    let mut handler_list: HandlerList = Arc::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Auth {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Echo {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Text {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Relationship {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Group {}));
    let new_connection_handler_generator: NewConnectionHandlerGenerator = Box::new(move || {
        Box::new(ClientConnectionHandler::new(
            handler_list.clone(),
            outer_channel.0.clone(),
        ))
    });
    let address = CONFIG.server.cluster_address;
    if address.is_ipv6() && !is_ipv6_enabled() {
        panic!("ipv6 is not enabled on this machine");
    }
    let mut server_config_builder = ServerConfigBuilder::default();
    server_config_builder
        .with_address(CONFIG.server.cluster_address)
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
        let res = io_tasks(outer_channel.1).await;
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
    Ok(())
}
