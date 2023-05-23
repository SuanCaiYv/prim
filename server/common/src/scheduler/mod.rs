use std::{sync::Arc, time::Duration};

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo, ServerType},
    net::{
        client::{ClientConfig, ClientReqwest},
        NewReqwestConnectionHandler, ReqwestHandlerGenerator, ReqwestHandlerMap,
        ReqwestOperatorManager,
    },
    Result,
};
use tokio::sync::mpsc;
use tracing::error;

pub async fn connect2scheduler(
    client_config: ClientConfig,
    client_id: u32,
    timeout: Duration,
    handler_map: ReqwestHandlerMap,
    self_info: ServerInfo,
) -> Result<ReqwestOperatorManager> {
    let mut client = ClientReqwest::new(client_config, timeout, client_id);

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
                            error!("no handler for resource_id: {}", resource_id);
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
    }
    let generator: ReqwestHandlerGenerator =
        Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
            Box::new(ReqwestMessageHandler {
                handler_map: handler_map.clone(),
            })
        });
    let generator = Arc::new(generator);
    let operator = client.build(generator).await?;

    let mut auth_info = self_info.clone();
    auth_info.typ = ServerType::SchedulerClient;
    let auth_msg = ReqwestMsg::with_resource_id_payload(
        ReqwestResourceID::NodeAuth.value(),
        &auth_info.to_bytes(),
    );
    let resp = operator.call(auth_msg).await?;
    if resp.payload() != b"true" {
        return Err(anyhow!("auth failed"));
    }
    let register_msg = ReqwestMsg::with_resource_id_payload(
        ReqwestResourceID::SeqnumNodeRegister.value(),
        &self_info.to_bytes(),
    );
    let resp = operator.call(register_msg).await?;
    if resp.payload() != b"true" {
        return Err(anyhow!("register failed"));
    }
    Ok(operator)
}
