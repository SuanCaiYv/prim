use ahash::AHashMap;
use anyhow::anyhow;
use async_channel::Receiver;
use async_trait::async_trait;
use quinn::{Connection, IncomingBiStreams, IncomingUniStreams, RecvStream, SendStream};
use crate::entity::Msg;

pub type Result<T> = anyhow::Result<T>;
pub type LenBuffer = [u8; 4];
pub type HandlerList = Box<dyn Handler>;
pub type GenericParameterMap = AHashMap<&'static str, Box<dyn GenericParameter>>;

pub trait GenericParameter {
    fn as_any(&self) -> &dyn std::any::Any;
}

pub fn get_parameter<T: GenericParameter>(parameters: &GenericParameterMap) -> Result<&T> {
    let parameter = parameters.get(std::any::type_name::<T>());
    if parameter.is_none() {
        Err(anyhow!("parameter not found"))
    } else {
        let parameter = parameter.unwrap();
        let parameter = parameter.as_any().downcast_ref::<T>();
        if parameter.is_none() {
            Err(anyhow!("parameter type mismatch"))
        } else {
            Ok(parameter.unwrap())
        }
    }
}

pub fn get_parameter_mut<T: GenericParameter>(parameters: &mut GenericParameterMap) -> Result<&mut T> {
    let parameter = parameters.get_mut(std::any::type_name::<T>());
    if parameter.is_none() {
        Err(anyhow!("parameter not found"))
    } else {
        let parameter = parameter.unwrap();
        let parameter = parameter.as_any().downcast_mut::<T>();
        if parameter.is_none() {
            Err(anyhow!("parameter type mismatch"))
        } else {
            Ok(parameter.unwrap())
        }
    }
}

/// a parameter struct passed to handler function to avoid repeated construction of some singleton variable.
pub struct HandlerParameters {
    #[allow(unused)]
    pub buffer: LenBuffer,
    #[allow(unused)]
    pub streams: (SendStream, RecvStream),
    #[allow(unused)]
    pub outer_stream: Receiver<Msg>,
    #[allow(unused)]
    pub generic_parameters: GenericParameterMap,
}

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    // the shared part is the function, not the data. So the `self` should be immutable.
    async fn handle_function(
        &self,
        msg: &mut Msg,
        parameters: &mut HandlerParameters,
    ) -> Result<Msg>;
}

#[async_trait]
pub trait ConnectionTask {
    fn new(conn: Connection, streams: (IncomingBiStreams, IncomingUniStreams), handler_list: HandlerList);

    async fn handle(&mut self) -> Result<()>;

    async fn read_msg(parameters: &mut HandlerParameters) -> Result<Msg>;

    async fn write_msg(parameters: &mut HandlerParameters, msg: &Msg) -> Result<()>;
}