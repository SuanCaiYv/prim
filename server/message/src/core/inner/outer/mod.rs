mod handler;
mod client;

use std::sync::Arc;
use common::net::InnerSender;

use common::net::server::HandlerList;
use common::Result;
use self::handler::internal::Balancer;

pub(crate) async fn start(inner_sender: InnerSender) -> Result<()> {
    let mut handler_list: HandlerList = Arc::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Balancer {}));
    let client = client::ClientToBalancer { io_handler_sender: inner_sender, handler_list };
    client.run().await?;
    Ok(())
}