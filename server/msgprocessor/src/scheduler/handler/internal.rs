use async_trait::async_trait;
use lib_net_tokio::net::ReqwestHandler;
use rdkafka::{
    consumer::{CommitMode, Consumer, StreamConsumer},
    ClientConfig, Message,
};
use tracing::{error, info};

use lib::{
    entity::{Msg, ReqwestMsg},
    error::HandlerError,
    net::InnerStates,
    Result,
};

use crate::config::CONFIG;

pub(crate) struct AssignProcessor {}

#[async_trait]
impl ReqwestHandler for AssignProcessor {
    /// the params is a json object, it contains the following fields:
    /// - `id`: the id of the message node
    /// - `topic`: the topic of the message node
    /// - `consumer_number`: the number of consumer of the message node
    async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
        let params = serde_json::from_slice::<serde_json::Value>(msg.payload())?;
        params
            .as_object()
            .ok_or(HandlerError::Parse("parse params error".to_string()))?;
        let id = params["id"].as_u64().unwrap();
        let topic = params["topic"].as_str().unwrap();
        let consumer_number = params["consumer_number"].as_u64().unwrap();
        info!(
            "start consumer group for message id: {}, topic: {}, consumer_number: {}",
            id, topic, consumer_number
        );
        for _ in 0..consumer_number + 1 {
            let consumer: StreamConsumer = match ClientConfig::new()
                .set("group.id", &format!("{}-default", topic))
                .set("bootstrap.servers", &CONFIG.message_queue.address)
                .set("enable.partition.eof", "false")
                .set("session.timeout.ms", "6000")
                .set("enable.auto.commit", "false")
                .create()
            {
                Ok(consumer) => consumer,
                Err(e) => {
                    error!("consumer creation failed: {}", e);
                    continue;
                }
            };
            if let Err(e) = consumer.subscribe(&["msg-test"]) {
                error!("subscribe error: {}", e);
                continue;
            }
            tokio::spawn(async move {
                loop {
                    match consumer.recv().await {
                        Err(e) => {
                            error!("kafka consumer recv error: {}", e.to_string());
                            continue;
                        }
                        Ok(msg) => {
                            match msg.payload() {
                                Some(bytes) => {
                                    let msg = Msg(bytes.to_owned());
                                }
                                None => {
                                    error!("kafka consumer recv error: payload is none");
                                    continue;
                                }
                            }
                            if let Err(e) = consumer.commit_message(&msg, CommitMode::Sync) {
                                error!("kafka consumer commit error: {}", e.to_string());
                                continue;
                            }
                        }
                    }
                }
            });
        }
        Ok(ReqwestMsg::default())
    }
}

pub(crate) struct UnassignProcessor {}

#[async_trait]
impl ReqwestHandler for UnassignProcessor {
    async fn run(&self, _msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
        Ok(ReqwestMsg::default())
    }
}
