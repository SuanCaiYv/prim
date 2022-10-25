use ahash::AHashMap;
use std::sync::Arc;

use async_trait::async_trait;
use common::entity::Msg;
use common::error::HandlerError;
use jwt_simple::reexports::anyhow::anyhow;
use quinn::{NewConnection, RecvStream, SendStream, VarInt};
use tokio::select;
use tracing::{debug, error, info};

use crate::cache::get_redis_ops;
use crate::cache::redis_ops::RedisOps;
use crate::core::{get_connection_map, ConnectionMap};
use crate::CONFIG;
use common::net::server::{
    ConnectionTask, ConnectionUtil, GenericParameterMap, HandlerList, HandlerParameters,
};
use common::net::{InnerReceiver, InnerSender, MsgIO};
use common::Result;

/// provide some external information.
#[allow(unused)]
pub(super) struct MessageConnectionTask {
    #[allow(unused)]
    pub(super) connection: NewConnection,
    #[allow(unused)]
    pub(super) handler_list: HandlerList,
    #[allow(unused)]
    pub(super) global_sender: InnerSender,
}

impl MessageConnectionTask {
    #[allow(unused)]
    fn new(
        connection: NewConnection,
        handler_list: HandlerList,
        global_sender: InnerSender,
    ) -> MessageConnectionTask {
        MessageConnectionTask {
            connection,
            handler_list,
            global_sender,
        }
    }

    /// this method return an error means the connection is not authed.
    #[inline]
    async fn first_read(
        handler_list: &HandlerList,
        parameters: &mut HandlerParameters,
        to: async_channel::Sender<Arc<Msg>>,
    ) -> Result<()> {
        let auth = &handler_list[0];
        let msg = MsgIO::read_msg(&mut parameters.buffer, &mut parameters.streams.1).await?;
        debug!("first read task read msg: {}", msg);
        let res = auth.run(msg.clone(), parameters).await;
        if let Ok(_) = res {
            let map = parameters
                .generic_parameters
                .get_parameter_mut::<ConnectionMap>();
            if map.is_ok() {
                map.unwrap().0.insert(msg.sender(), to);
            } else {
                return Err(anyhow!("connection map not found."));
            }
            MsgIO::write_msg(
                Arc::new(msg.generate_ack(msg.timestamp())),
                &mut parameters.streams.0,
            )
            .await?;
        } else {
            // auth failed, so close the outer connection.
            to.close();
            MsgIO::write_msg(
                Arc::new(Msg::err_msg_str(0, msg.sender(), "auth failed.")),
                &mut parameters.streams.0,
            )
            .await?;
            // give that error response and finish the stream.
            let _ = parameters.streams.0.finish().await;
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
        Self::epoll_stream(handler_list, &mut parameters).await?;
        Ok(())
    }

    /// this method never return errors.
    #[allow(unused)]
    async fn new_stream_task(
        handler_list: HandlerList,
        to: InnerSender,
        mut from: InnerReceiver,
        (mut send, mut recv): (SendStream, RecvStream),
    ) -> Result<()> {
        let mut parameters = HandlerParameters {
            buffer: [0; 4],
            streams: (send, recv),
            inner_streams: (to, from),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        Self::epoll_stream(handler_list, &mut parameters).await?;
        Ok(())
    }

    /// this method will never return an error. when it returned, that means this stream should be closed.
    #[allow(unused)]
    #[inline]
    async fn epoll_stream(
        handler_list: HandlerList,
        parameters: &mut HandlerParameters,
    ) -> Result<()> {
        loop {
            select! {
                msg = parameters.inner_streams.1.recv() => {
                    if let Ok(mut msg) = msg {
                        let res = MsgIO::write_msg(msg, &mut parameters.streams.0).await;
                        if res.is_err() {
                            break;
                        }
                    } else {
                        info!("outer stream closed.");
                        break;
                    }
                },
                msg = MsgIO::read_msg(&mut parameters.buffer, &mut parameters.streams.1) => {
                    if let Ok(mut msg) = msg {
                        info!("read msg: {}", msg);
                        parameters.inner_streams.0.send(msg.clone()).await;
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
        parameters.streams.0.finish().await?;
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
        MsgIO::write_msg(Arc::new(res_msg.unwrap()), &mut parameters.streams.0).await?;
        Ok(())
    }
}

#[async_trait]
impl ConnectionTask for MessageConnectionTask {
    async fn handle(mut self: Box<Self>) -> Result<()> {
        let Self {
            mut connection,
            handler_list,
            global_sender,
        } = *self;
        let (to, from) =
            async_channel::bounded(CONFIG.performance.max_outer_connection_channel_buffer_size);
        // the first stream and first msg should be `auth` msg.
        // when the first work, any error should shutdown the connection
        let first_stream = ConnectionUtil::first_stream(&mut connection).await?;
        debug!("get first stream successfully");
        let handler_list0 = handler_list.clone();
        let from0 = from.clone();
        let mut parameters = HandlerParameters {
            buffer: [0; 4],
            streams: first_stream,
            inner_streams: (global_sender.clone(), from0),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        parameters
            .generic_parameters
            .put_parameter::<ConnectionMap>(get_connection_map());
        parameters
            .generic_parameters
            .put_parameter::<RedisOps>(get_redis_ops().await);
        let res = Self::first_read(&handler_list0, &mut parameters, to).await;
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
            let stream = ConnectionUtil::more_stream(&mut connection).await;
            if stream.is_err() {
                break;
            }
            let stream = stream.unwrap();
            let handler_list = handler_list.clone();
            let from = from.clone();
            let to = global_sender.clone();
            tokio::spawn(async move {
                info!("new stream task");
                let _ = Self::new_stream_task(handler_list, to, from, stream).await;
            });
        }
        // no more streams arrived, so this connection should be closed normally.
        connection
            .connection
            .close(VarInt::from(0_u8), "connection done.".as_bytes());
        info!("connection done.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() {}
}
