use ahash::AHashMap;
use lib::{
    entity::ReqwestMsg,
    error::HandlerError,
    net::server::{GenericParameterMap, HandlerParameters, InnerStates, ReqwestHandlerList},
    Result,
};
use tokio::sync::mpsc;
use tracing::{debug, error};

pub(super) mod logic;

pub(super) async fn handler_func(
    sender: mpsc::Sender<ReqwestMsg>,
    mut receiver: mpsc::Receiver<ReqwestMsg>,
    handler_list: &ReqwestHandlerList,
    inner_states: &mut InnerStates,
) -> Result<()> {
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    loop {
        let msg = receiver.recv().await;
        match msg {
            Some(msg) => {
                call_handler_list(
                    &sender,
                    msg,
                    handler_list,
                    &mut handler_parameters,
                    inner_states,
                )
                .await?;
            }
            None => {
                debug!("connection closed");
                break;
            }
        }
    }
    Ok(())
}

/// this function is used to deal with logic/business message received from client.
#[inline(always)]
pub(crate) async fn call_handler_list(
    sender: &mpsc::Sender<ReqwestMsg>,
    msg: ReqwestMsg,
    handler_list: &ReqwestHandlerList,
    handler_parameters: &mut HandlerParameters,
    inner_states: &mut InnerStates,
) -> Result<()> {
    for handler in handler_list.iter() {
        match handler.run(&msg, handler_parameters, inner_states).await {
            Ok(ok_msg) => {
                sender.send(ok_msg).await?;
                break;
            }
            Err(e) => {
                match e.downcast::<HandlerError>() {
                    Ok(handler_err) => match handler_err {
                        HandlerError::NotMine => {
                            continue;
                        }
                        _ => {
                            error!("handler error: {}", handler_err);
                            break;
                        }
                    },
                    Err(e) => {
                        error!("unhandled error: {}", e);
                        break;
                    }
                };
            }
        }
    }
    Ok(())
}
