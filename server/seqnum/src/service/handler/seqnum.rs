use async_trait::async_trait;
use lib::{
    entity::ReqwestMsg,
    net::{InnerStates, ReqwestHandler},
    Result,
};

pub(crate) struct SeqNum;

#[async_trait]
impl ReqwestHandler for SeqNum {
    async fn run(
        &self,
        msg: &mut ReqwestMsg,
        _inner_states: &mut InnerStates,
    ) -> Result<ReqwestMsg> {
        Ok(msg.clone())
    }
}
