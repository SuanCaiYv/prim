use crate::cache::get_redis_ops;
use crate::config::CONFIG;
use crate::inner::{get_connection_map, get_status_map, ConnectionId, ConnectionMap, StatusMap};
use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use common::cache::redis_ops::RedisOps;
use common::entity::{Msg, NodeStatus, Type};
use common::error::HandlerError;
use common::net::server::{
    ConnectionTask, ConnectionUtil, GenericParameterMap, HandlerList, HandlerParameters,
};
use common::net::{InnerReceiver, InnerSender, MsgIO, OuterSender};
use common::Result;
use quinn::{NewConnection, RecvStream, SendStream, VarInt};
use std::sync::Arc;
use tokio::select;
use tracing::{debug, error, info};

/// provide some external information.
#[allow(unused)]
pub(super) struct BalancerConnectionTask {
    #[allow(unused)]
    pub(super) connection: NewConnection,
    #[allow(unused)]
    pub(super) handler_list: HandlerList,
    #[allow(unused)]
    pub(super) inner_sender: InnerSender,
}

impl BalancerConnectionTask {
    #[allow(unused)]
    fn new(
        connection: NewConnection,
        handler_list: HandlerList,
        inner_sender: InnerSender,
    ) -> BalancerConnectionTask {
        BalancerConnectionTask {
            connection,
            handler_list,
            inner_sender,
        }
    }

    /// this method return an error means the connection is not authed.
    #[inline]
    async fn first_read(
        connection_id: u64,
        handler_list: &HandlerList,
        parameters: &mut HandlerParameters,
        bridge_sender: OuterSender,
    ) -> Result<()> {
        let auth = &handler_list[0];
        let msg = MsgIO::read_msg(&mut parameters.buffer, &mut parameters.net_streams.1).await?;
        debug!("first read task read msg: {}", msg);
        let res = auth.run(msg.clone(), parameters).await;
        if let Ok(_) = res {
            let connection_map = parameters
                .generic_parameters
                .get_parameter_mut::<ConnectionMap>();
            if connection_map.is_ok() {
                connection_map
                    .unwrap()
                    .0
                    .insert(msg.sender(), bridge_sender);
            } else {
                return Err(anyhow!("connection map not found."));
            }
            MsgIO::write_msg(
                Arc::new(msg.generate_ack(msg.timestamp())),
                &mut parameters.net_streams.0,
            )
            .await?;
        } else {
            // auth failed, so close the outer connection.
            bridge_sender.close();
            MsgIO::write_msg(
                Arc::new(Msg::err_msg_str(0, msg.sender(), "auth failed.")),
                &mut parameters.net_streams.0,
            )
            .await?;
            // give that error response and finish the stream.
            let _ = parameters.net_streams.0.finish().await;
            info!("first read with auth failed: {}", res.err().unwrap());
            return Err(anyhow!("auth failed."));
        }
        Ok(())
    }

    #[inline]
    async fn first_stream_task(
        handler_list: HandlerList,
        mut parameters: HandlerParameters,
    ) -> Result<()> {
        Self::poll_stream(handler_list, &mut parameters).await?;
        Ok(())
    }

    /// this method never return errors.
    #[allow(unused)]
    async fn new_stream_task(
        connection_id: u64,
        handler_list: HandlerList,
        inner_channel: (InnerSender, InnerReceiver),
        net_streams: (SendStream, RecvStream),
    ) -> Result<()> {
        let mut parameters = HandlerParameters {
            buffer: [0; 4],
            net_streams,
            inner_channel,
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        parameters
            .generic_parameters
            .put_parameter::<StatusMap>(get_status_map());
        parameters
            .generic_parameters
            .put_parameter::<RedisOps>(get_redis_ops().await);
        parameters
            .generic_parameters
            .put_parameter::<ConnectionId>(ConnectionId(connection_id));
        Self::poll_stream(handler_list, &mut parameters).await?;
        Ok(())
    }

    /// this method will never return an error. when it returned, that means this stream should be closed.
    #[allow(unused)]
    #[inline]
    async fn poll_stream(
        handler_list: HandlerList,
        parameters: &mut HandlerParameters,
    ) -> Result<()> {
        loop {
            let inner_sender = &parameters.inner_channel.0;
            let inner_receiver = &parameters.inner_channel.1;
            select! {
                msg = inner_receiver.recv() => {
                    if let Ok(mut msg) = msg {
                        let res = MsgIO::write_msg(msg, &mut parameters.net_streams.0).await;
                        if res.is_err() {
                            break;
                        }
                    } else {
                        info!("outer stream closed.");
                        break;
                    }
                },
                msg = MsgIO::read_msg(&mut parameters.buffer, &mut parameters.net_streams.1) => {
                    if let Ok(mut msg) = msg {
                        inner_sender.send(msg.clone()).await;
                        let res = Self::handle_msg(&handler_list, msg, parameters).await;
                        if res.is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        parameters.net_streams.0.finish().await?;
        Ok(())
    }

    /// the only error returned indicates that the stream is closed.
    #[allow(unused)]
    #[inline]
    async fn handle_msg(
        handler_list: &HandlerList,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
    ) -> Result<()> {
        let mut res_msg: Option<Msg> = None;
        for handler in handler_list.iter() {
            let res = handler.run(msg.clone(), parameters).await;
            if let Ok(success) = res {
                res_msg = Some(success);
            } else {
                let err = res.err().unwrap().downcast::<HandlerError>();
                if err.is_err() {
                    error!("unhandled error: {}", err.as_ref().err().unwrap());
                    continue;
                }
                match err.unwrap() {
                    HandlerError::NotMine => {
                        continue;
                    }
                    HandlerError::Auth { .. } => {
                        let msg = Msg::err_msg_str(0, msg.sender(), "auth failed.");
                        res_msg = Some(msg);
                        break;
                    }
                    HandlerError::Parse(cause) => {
                        let msg = Msg::err_msg(0, msg.sender(), cause);
                        res_msg = Some(msg);
                        break;
                    }
                }
            }
        }
        if res_msg.is_none() {
            let msg = Msg::err_msg_str(0, msg.sender(), "no handler found.");
            res_msg = Some(msg);
        }
        MsgIO::write_msg(Arc::new(res_msg.unwrap()), &mut parameters.net_streams.0).await?;
        Ok(())
    }
}

#[async_trait]
impl ConnectionTask for BalancerConnectionTask {
    async fn handle(mut self: Box<Self>) -> Result<()> {
        let Self {
            mut connection,
            handler_list,
            inner_sender,
        } = *self;
        let connection_id = connection.connection.stable_id() as u64;
        let (bridge_sender, bridge_receiver) =
            async_channel::bounded(CONFIG.performance.max_outer_connection_channel_buffer_size);
        // the first stream and first msg should be `auth` msg.
        // when the first work, any error should shutdown the connection
        let first_stream = ConnectionUtil::first_stream(&mut connection).await?;
        debug!("get first stream successfully");
        let handler_list0 = handler_list.clone();
        let mut parameters = HandlerParameters {
            buffer: [0; 4],
            net_streams: first_stream,
            inner_channel: (inner_sender.clone(), bridge_receiver.clone()),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        parameters
            .generic_parameters
            .put_parameter::<ConnectionMap>(get_connection_map());
        parameters
            .generic_parameters
            .put_parameter::<StatusMap>(get_status_map());
        parameters
            .generic_parameters
            .put_parameter::<RedisOps>(get_redis_ops().await);
        parameters
            .generic_parameters
            .put_parameter::<ConnectionId>(ConnectionId(connection_id));
        let res = Self::first_read(
            connection_id,
            &handler_list0,
            &mut parameters,
            bridge_sender,
        )
        .await;
        if res.is_err() {
            connection
                .connection
                .close(VarInt::from(1_u8), b"first read failed.");
            return Err(anyhow!("first read fatal."));
        }
        tokio::spawn(async move {
            let _ = Self::first_stream_task(handler_list0, parameters).await;
        });
        loop {
            let streams = ConnectionUtil::more_stream(&mut connection).await;
            if streams.is_err() {
                break;
            }
            let streams = streams.unwrap();
            let handler_list = handler_list.clone();
            let inner_receiver = bridge_receiver.clone();
            let inner_sender = inner_sender.clone();
            tokio::spawn(async move {
                info!("new stream task");
                let _ = Self::new_stream_task(
                    connection_id,
                    handler_list,
                    (inner_sender, inner_receiver),
                    streams,
                )
                .await;
            });
        }
        // no more streams arrived, so this connection should be closed normally.
        connection
            .connection
            .close(VarInt::from(0_u8), "connection done.".as_bytes());
        let status_map = get_status_map();
        let node_info = status_map.0.get(&connection_id);
        if let Some(node_info) = node_info {
            node_info.status = NodeStatus::DirectUnregister;
            let mut msg = Msg::raw_payload(&node_info.to_bytes());
            msg.update_type(Type::NodeUnregister);
            inner_sender.send(Arc::new(msg)).await?;
        }
        status_map.0.remove(&connection_id);
        info!("connection done.");
        Ok(())
    }
}
