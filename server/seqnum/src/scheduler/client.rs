use std::time::Duration;

use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
    net::client::ClientConfigBuilder,
    Result,
};
use lib_net_monoio::net::{client::ClientReqwestTcp, ReqwestOperatorManager};

use crate::{config::CONFIG, util::my_id};

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<ReqwestOperatorManager> {
        let scheduler_address = CONFIG.scheduler.address;

        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(scheduler_address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let client_config = config_builder.build().unwrap();

        let mut service_address = CONFIG.server.service_address;
        service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(service_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::SeqnumCluster,
            load: None,
        };

        let mut client = ClientReqwestTcp::new(client_config, Duration::from_millis(3000));
        let operator = client.build().await?;
        let mut auth_info = server_info.clone();
        auth_info.typ = ServerType::SchedulerClient;
        let auth_msg = ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::NodeAuth,
            &auth_info.to_bytes(),
        );
        let _resp = operator.call(auth_msg).await?;
        let register_msg = ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::SeqnumNodeRegister,
            &server_info.to_bytes(),
        );
        let _resp = operator.call(register_msg).await?;
        Box::leak(Box::new(client));
        Ok(operator)
    }
}
