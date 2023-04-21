use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use std::time::Instant;

use ahash::AHashMap;
use anyhow::Context;
use async_trait::async_trait;
use tokio::sync::Mutex;
use lib::{
    entity::ReqwestMsg,
    joy,
    net::{
        client::{ClientConfigBuilder, ClientReqwest},
        server::{
            GenericParameterMap, HandlerParameters, InnerStates, NewReqwestConnectionHandler,
            NewReqwestConnectionHandlerGenerator, ReqwestHandler, ReqwestHandlerList,
            ServerConfigBuilder, ServerReqwest,
        },
        ReqwestMsgIOWrapper,
    },
    Result,
};

use tracing::{error, info};

use crate::config::CONFIG;

mod cache;
mod config;
mod util;

struct Echo {}

#[async_trait]
impl ReqwestHandler for Echo {
    async fn run(
        &self,
        msg: &ReqwestMsg,
        _parameters: &mut HandlerParameters,
        // this one contains some states corresponding to the quic stream.
        _inner_states: &mut InnerStates,
    ) -> Result<ReqwestMsg> {
        let req_id = msg.req_id();
        let resource_id = msg.resource_id();
        let mut number = String::from_utf8_lossy(msg.payload())
            .to_string()
            .parse::<u64>();
        if number.is_err() {
            error!("failed num: {}", String::from_utf8_lossy(msg.payload()));
            number = Ok(111);
        }
        let number = number.unwrap();
        let resp = format!(
            "hello client, you are:{:06} have required for {:06} with {:06}.",
            req_id, resource_id, number
        );
        let mut resp_msg = ReqwestMsg::with_resource_id_payload(resource_id, resp.as_bytes());
        resp_msg.set_req_id(req_id);
        Ok(resp_msg)
    }
}

struct ReqwestMessageHandler {
    handler_list: ReqwestHandlerList,
}

#[async_trait]
impl NewReqwestConnectionHandler for ReqwestMessageHandler {
    async fn handle(&mut self, mut io_operators: ReqwestMsgIOWrapper) -> Result<()> {
        let (sender, mut receiver) = io_operators.channels();
        let mut parameters = HandlerParameters {
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        let mut inner_states = InnerStates::new();
        loop {
            let msg = receiver.recv().await;
            match msg {
                Some(msg) => {
                    let resp = self.handler_list[0]
                        .run(&msg, &mut parameters, &mut inner_states)
                        .await;
                    match resp {
                        Ok(resp) => {
                            let _ = sender.send(resp).await;
                        }
                        Err(e) => {
                            println!("error: {}", e);
                        }
                    }
                }
                None => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(CONFIG.log_level)
        .try_init()
        .unwrap();
    println!("{}", joy::banner());
    let mut server_config_builder = ServerConfigBuilder::default();
    server_config_builder.with_address("0.0.0.0:8190".parse().unwrap());
    server_config_builder.with_connection_idle_timeout(3000);
    server_config_builder.with_max_bi_streams(8);
    server_config_builder.with_max_connections(100);
    server_config_builder.with_cert(rustls::Certificate(
        fs::read(PathBuf::from(
            "/Users/slma/RustProjects/prim/server/cert/localhost-server.crt.der",
        ))
        .context("read cert file failed.")
        .unwrap(),
    ));
    server_config_builder.with_key(rustls::PrivateKey(
        fs::read(PathBuf::from(
            "/Users/slma/RustProjects/prim/server/cert/localhost-server.key.der",
        ))
        .context("read key file failed.")
        .unwrap(),
    ));
    let mut client_config_builder = ClientConfigBuilder::default();
    client_config_builder.with_remote_address("127.0.0.1:8190".parse().unwrap());
    client_config_builder.with_domain("localhost".to_string());
    client_config_builder.with_ipv4_type(true);
    client_config_builder.with_max_bi_streams(8);
    client_config_builder.with_keep_alive_interval(Duration::from_millis(2000));
    client_config_builder.with_cert(rustls::Certificate(
        fs::read(PathBuf::from(
            "/Users/slma/RustProjects/prim/server/cert/PrimRootCA.crt.der",
        ))
        .context("read cert file failed.")
        .unwrap(),
    ));
    let server_config = server_config_builder.build().unwrap();
    let client_config = client_config_builder.build().unwrap();
    let mut handler_list: Vec<Box<dyn ReqwestHandler>> = Vec::new();
    handler_list.push(Box::new(Echo {}));
    let handler_list = ReqwestHandlerList::new(handler_list);
    let generator: NewReqwestConnectionHandlerGenerator = Box::new(move || {
        Box::new(ReqwestMessageHandler {
            handler_list: handler_list.clone(),
        })
    });
    let mut server = ServerReqwest::new(server_config);
    let mut client = ClientReqwest::new(client_config);
    let holder = Arc::new(Mutex::new(None));
    let holder2 = holder.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        client.build().await.unwrap();
        let client = Arc::new(client);
        for i in 0..2000 {
            let client = client.clone();
            tokio::spawn(async move {
                let resource_id = fastrand::u16(..);
                let req = ReqwestMsg::with_resource_id_payload(resource_id, format!("{}", i).as_bytes());
                let resp = client.call(req).unwrap();
                let resp = resp.await.unwrap();
                info!("resp: {}", String::from_utf8_lossy(resp.payload()));
            });
        }
        holder2.lock().await.replace(client);
    });
    if let Err(e) = server.run(generator).await {
        error!("message server error: {}", e);
    }
    Ok(())
}
