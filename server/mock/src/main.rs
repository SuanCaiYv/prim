use std::{
    println,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use ahash::AHashMap;
use async_trait::async_trait;
use lib::{Result, entity::{ReqwestMsg, ReqwestResourceID}, net::{InnerStates, server::ServerConfigBuilder, client::ClientConfigBuilder}, joy};

use lib_net_tokio::net::{ReqwestHandler, ReqwestHandlerMap, NewReqwestConnectionHandler, ReqwestHandlerGenerator, server::{ServerReqwest, ReqwestCaller}, client::ClientReqwest};
use tokio::sync::mpsc;
use tracing::error;

use crate::config::CONFIG;

mod cache;
mod config;
mod util;

struct Echo {}

#[async_trait]
impl ReqwestHandler for Echo {
    async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
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
        // Ok(msg.clone())
    }
}

struct ReqwestMessageHandler {
    handler_map: ReqwestHandlerMap,
}

#[async_trait]
impl NewReqwestConnectionHandler for ReqwestMessageHandler {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>),
    ) -> Result<()> {
        let mut states = AHashMap::new();
        let (send, mut recv) = msg_operators;
        loop {
            match recv.recv().await {
                Some(mut msg) => {
                    let resource_id = msg.resource_id();
                    let handler = self.handler_map.get(&resource_id);
                    if handler.is_none() {
                        error!(
                            "no handler for resource_id: {}, {}",
                            resource_id,
                            msg.req_id()
                        );
                        continue;
                    }
                    let handler = handler.unwrap();
                    let resp = handler.run(&mut msg, &mut states).await;
                    if resp.is_err() {
                        error!("handler run error: {}", resp.err().unwrap());
                        continue;
                    }
                    let resp = resp.unwrap();
                    let _ = send.send(resp).await;
                }
                None => {
                    break;
                }
            }
        }
        Ok(())
    }

    fn set_reqwest_caller(&mut self, _client_caller: ReqwestCaller) {}
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
    let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
    handler_map.insert(ReqwestResourceID::Ping, Box::new(Echo {}));
    let handler_map = ReqwestHandlerMap::new(handler_map);
    let generator: ReqwestHandlerGenerator = Box::new(move || {
        Box::new(ReqwestMessageHandler {
            handler_map: handler_map.clone(),
        })
    });
    let generator = Arc::new(generator);
    let mut server = ServerReqwest::new(server_config, Duration::from_millis(5000));
    let mut client = ClientReqwest::new(client_config, Duration::from_millis(5000));
    let gen = generator.clone();
    tokio::spawn(async move {
        let time_elapsed = Arc::new(AtomicU64::new(0));
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let operator_manager = client.build(gen).await?;
        let operator_manager = Arc::new(operator_manager);
        // about 5µs per call, or 230K QPS
        let n = 230000;
        for i in 0..n {
            let operator_manager = operator_manager.clone();
            let elapsed = time_elapsed.clone();
            tokio::spawn(async move {
                let req = ReqwestMsg::with_resource_id_payload(ReqwestResourceID::Ping, format!("{:06}", i).as_bytes());
                let req = operator_manager.call(req);
                // let t = Instant::now();
                match req.await {
                    Ok(_resp) => {
                        // info!("resp: {}", String::from_utf8_lossy(_resp.payload()));
                        elapsed.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => {
                        error!("call error: {}", e);
                    }
                }
            });
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        // println!("total cost: {} ms", time_elapsed.load(Ordering::SeqCst));
        // println!("avg cost: {} ms", time_elapsed.load(Ordering::SeqCst) / n);
        println!("total done {}", time_elapsed.load(Ordering::SeqCst));
        Result::<()>::Ok(())
    });
    if let Err(e) = server.run(generator).await {
        error!("reqwest server error: {}", e);
    }
    Ok(())
}
