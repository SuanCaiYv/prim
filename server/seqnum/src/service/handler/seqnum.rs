use std::sync::{atomic::AtomicU64, Arc};

use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use lib::{
    entity::ReqwestMsg,
    net::{InnerStates, ReqwestHandler},
    Result,
};

use crate::{service::SeqnumMap, config::CONFIG, persistence};

pub(crate) struct SeqNum;

#[async_trait]
impl ReqwestHandler for SeqNum {
    async fn run(
        &self,
        msg: &mut ReqwestMsg,
        states: &mut InnerStates,
    ) -> Result<ReqwestMsg> {
        let key = BigEndian::read_u128(msg.payload());
        let generic_map = states.get("generic_map").unwrap().as_generic_parameter_map().unwrap();
        let seqnum_op = match generic_map.get_parameter::<SeqnumMap>()?.get(&key) {
            Some(seqnum) => {
                (*seqnum).clone()
            }
            None => {
                let seqnum = Arc::new(AtomicU64::new(0));
                generic_map.get_parameter::<SeqnumMap>()?.insert(key, seqnum.clone());
                seqnum
            }
        };
        let seqnum = seqnum_op.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        if CONFIG.server.exactly_mode {
            persistence::save(key, seqnum).await?;
        } else {
            if seqnum & 0x7F == 0 {
                persistence::save(key, seqnum).await?;
            }
        };
        let mut buf = [0u8; 8];
        BigEndian::write_u64(&mut buf, seqnum);
        Ok(ReqwestMsg::with_resource_id_payload(msg.resource_id(), &buf))
    }
}
