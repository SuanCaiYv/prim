use std::time::Duration;

use tracing::info;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
    net::client::ClientConfigBuilder,
    Result,
};
use lib_net_monoio::net::{client::ClientReqwestTcp, ReqwestOperatorManager};

use crate::{config::config, util::my_id};

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<ReqwestOperatorManager> {
        let scheduler_address = config().scheduler.address;
        info!("connecting to scheduler: {}...", scheduler_address);

        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(scheduler_address)
            .with_ipv4_type(config().server.cluster_address.is_ipv4())
            .with_domain(config().scheduler.domain.clone())
            .with_cert(config().scheduler.cert.clone())
            .with_keep_alive_interval(config().transport.keep_alive_interval)
            .with_max_bi_streams(config().transport.max_bi_streams);
        let client_config = config_builder.build().unwrap();

        let mut service_address = config().server.service_address;
        service_address.set_ip(config().server.service_ip.parse().unwrap());
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
        let mut operator: Option<ReqwestOperatorManager> = None;
        for _ in 0..5 {
            match client.build().await {
                Ok(operator0) => {
                    operator = Some(operator0);
                    break;
                }
                Err(e) => {
                    info!("connect to scheduler error: {}", e.to_string());
                    monoio::time::sleep(Duration::from_millis(2500)).await;
                },
            }
        };
        let operator = match operator {
            Some(operator) => operator,
            None => {
                return Err(anyhow::anyhow!("connect to scheduler error"));
            }
        };
        let mut auth_info = server_info.clone();
        auth_info.typ = ServerType::SchedulerClient;
        let auth_msg = ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::NodeAuth,
            &auth_info.to_bytes(),
        );
        let _resp = match operator.call(auth_msg).await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "auth to scheduler error: {}",
                    e.to_string()
                ));
            }
        };
        let register_msg = ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::SeqnumNodeRegister,
            &server_info.to_bytes(),
        );
        let _resp = match operator.call(register_msg).await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "register to scheduler error: {}",
                    e.to_string()
                ));
            }
        };
        Box::leak(Box::new(client));
        Ok(operator)
    }
}
