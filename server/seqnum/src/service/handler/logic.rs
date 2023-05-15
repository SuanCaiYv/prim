use async_trait::async_trait;
use lib::{
    entity::ReqwestMsg,
    net::server::{HandlerParameters, InnerStates, ReqwestHandler},
    Result,
};

pub(crate) struct SeqNum;

#[async_trait]
impl ReqwestHandler for SeqNum {
    async fn run(
        &self,
        msg: &ReqwestMsg,
        _parameters: &mut HandlerParameters,
        // this one contains some states corresponding to the quic stream.
        _inner_states: &mut InnerStates,
    ) -> Result<ReqwestMsg> {
        Ok(msg.clone())
    }
}
