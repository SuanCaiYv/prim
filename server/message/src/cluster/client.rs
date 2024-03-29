use std::{net::SocketAddr, sync::Arc};

use lib::{
    entity::{Msg, ServerInfo, ServerStatus, ServerType, Type},
    net::{client::ClientConfigBuilder, InnerStates},
    Result,
};
use lib_net_tokio::net::{
    client::{ClientMultiConnection, SubConnectionConfig},
    Handler, HandlerList,
};
use tracing::error;

use crate::{config::config, service::get_io_task_sender, util::my_id};

use super::{
    handler::{logger, logic, pure_text},
    MsgSender,
};

pub(super) struct Client {
    multi_client: ClientMultiConnection,
}

impl Client {
    pub(super) fn new() -> Self {
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address("[::1]:0".parse().unwrap())
            .with_ipv4_type(config().server.ipv4)
            .with_domain(config().server.domain.clone())
            .with_cert(config().server.cert.clone())
            .with_keep_alive_interval(config().transport.keep_alive_interval)
            .with_max_bi_streams(config().transport.max_bi_streams);
        let client_config = client_config.build().unwrap();
        let multi_client = ClientMultiConnection::new(client_config).unwrap();
        Self { multi_client }
    }

    pub(super) async fn new_connection(&self, remote_address: SocketAddr) -> Result<()> {
        let sub_config = SubConnectionConfig {
            remote_address,
            domain: config().server.domain.clone(),
            opened_bi_streams_number: config().transport.max_bi_streams,
            timeout: std::time::Duration::from_millis(3000),
        };
        let server_info = ServerInfo {
            id: my_id(),
            service_address: config().server.service_address.clone(),
            cluster_address: Some(config().server.cluster_address.clone()),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::MessageCluster,
            load: None,
        };
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        let mut conn = self
            .multi_client
            .new_connection(sub_config, Arc::new(auth))
            .await?;
        let (sender, receiver) = conn.operation_channel();
        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
        handler_list.push(Box::new(logic::ClientAuth {}));
        handler_list.push(Box::new(logger::Ack {}));
        handler_list.push(Box::new(pure_text::Text {}));
        let handler_list = HandlerList::new(handler_list);
        let io_task_sender = get_io_task_sender().clone();
        let mut inner_states = InnerStates::new();
        tokio::spawn(async move {
            // extend lifetime of connection
            let _conn = conn;
            if let Err(e) = super::handler::handler_func(
                MsgSender::Client(sender),
                receiver,
                &io_task_sender,
                &handler_list,
                &mut inner_states,
            )
            .await
            {
                error!("handler_func error: {}", e);
            }
        });
        Ok(())
    }
}
