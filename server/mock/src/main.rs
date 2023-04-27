use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
};

use ahash::AHashMap;

use async_trait::async_trait;
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

use tracing::error;

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
        // let req_id = msg.req_id();
        // let resource_id = msg.resource_id();
        // let mut number = String::from_utf8_lossy(msg.payload())
        //     .to_string()
        //     .parse::<u64>();
        // if number.is_err() {
        //     error!("failed num: {}", String::from_utf8_lossy(msg.payload()));
        //     number = Ok(111);
        // }
        // let number = number.unwrap();
        // let resp = format!(
        //     "hello client, you are:{:06} have required for {:06} with {:06}.",
        //     req_id, resource_id, number
        // );
        // let mut resp_msg = ReqwestMsg::with_resource_id_payload(resource_id, resp.as_bytes());
        // resp_msg.set_req_id(req_id);
        // Ok(resp_msg)
        Ok(msg.clone())
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
                    match self.handler_list[0]
                        .run(&msg, &mut parameters, &mut inner_states)
                        .await
                    {
                        Ok(resp) => {
                            if let Err(e) = sender.send(resp).await {
                                error!("{}", e);
                            }
                        }
                        Err(e) => {
                            error!("handler error: {}", e);
                        }
                    }
                }
                None => {
                    break;
                }
            };
        }
        Ok(())
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
    server_config_builder.with_cert(CONFIG.server.cert.clone());
    server_config_builder.with_key(CONFIG.server.key.clone());
    let mut client_config_builder = ClientConfigBuilder::default();
    client_config_builder.with_remote_address("127.0.0.1:8190".parse().unwrap());
    client_config_builder.with_domain("localhost".to_string());
    client_config_builder.with_ipv4_type(true);
    client_config_builder.with_max_bi_streams(8);
    client_config_builder.with_keep_alive_interval(Duration::from_millis(2000));
    client_config_builder.with_cert(CONFIG.scheduler.cert.clone());
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
    tokio::spawn(async move {
        let time_elapsed = Arc::new(AtomicU64::new(0));
        tokio::time::sleep(Duration::from_millis(1000)).await;
        _ = client.build().await;
        let client = Arc::new(client);
        let n = 4000;
        for i in 0..n {
            let client = client.clone();
            let elapsed = time_elapsed.clone();
            tokio::spawn(async move {
                let resource_id = fastrand::u16(..);
                let req = ReqwestMsg::with_resource_id_payload(
                    resource_id,
                    format!("{:06}", i).as_bytes(),
                );
                let req = client.call(req);
                let t = Instant::now();
                match req.await {
                    Ok(_resp) => {
                        // info!("resp: {}", String::from_utf8_lossy(resp.payload()));
                    }
                    Err(e) => {
                        error!("call error: {}", e);
                    }
                }
                elapsed.fetch_add(t.elapsed().as_millis() as u64, Ordering::SeqCst);
            });
        }
        tokio::time::sleep(Duration::from_millis(8000)).await;
        println!("avg cost: {} ms", time_elapsed.load(Ordering::SeqCst) / n);
    });
    if let Err(e) = server.run(generator).await {
        error!("reqwest server error: {}", e);
    }
    Ok(())
}
