use std::sync::Arc;

use lib::{Result, net::client::{ClientConfigBuilder, ClientTimeout}, entity::Msg};

use crate::{cache::{get_redis_ops, TOKEN_KEY}, util::jwt::simple_token, config::CONFIG};

pub(crate) async fn start() -> Result<()> {
    tokio::spawn(async move {
        _ = join_group(1, 68719476736, 1).await;
    });
    tokio::spawn(async move {
        _ = join_group(2, 68719476736, 1).await;
    });
    join_group(3, 68719476736, 1).await
}

pub(crate) async fn join_group(user_id: u64, group_id: u64, node_id: u32) -> Result<()> {
    let mut redis_ops = get_redis_ops().await;
    redis_ops
        .set(&format!("{}{}", TOKEN_KEY, user_id), &"aaa")
        .await?;
    let token = simple_token(b"aaa", user_id);
    let auth_msg = Msg::auth(user_id, 0, node_id, &token);
    let mut client_config = ClientConfigBuilder::default();
    client_config
        .with_remote_address(CONFIG.server.service_address)
        .with_domain(CONFIG.server.domain.clone())
        .with_cert(CONFIG.server.cert.clone())
        .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams)
        .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
        .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
    let config = client_config.build().unwrap();
    let mut client = ClientTimeout::new(config, std::time::Duration::from_millis(3000));
    client.run().await?;
    let (io_sender, mut io_receiver, mut timeout_receiver) = client.io_channel().await?;
    io_sender.send(Arc::new(auth_msg)).await?;
    io_receiver.recv().await;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = io_receiver.recv() => {
                    match msg {
                        Some(msg) => {
                            println!("user: {}, recv msg: {}", user_id, String::from_utf8_lossy(msg.payload()).to_string());
                        }
                        None => {
                            println!("recv msg: None");
                            break;
                        }
                    }
                }
                msg = timeout_receiver.recv() => {
                    match msg {
                        Some(msg) => {
                            println!("user: {}, recv timeout msg: {}", user_id, String::from_utf8_lossy(msg.payload()).to_string());
                        }
                        None => {
                            println!("recv timeout msg: None");
                            break;
                        }
                    }
                }
            }
        }
    });
    for i in 0..10 {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let msg = Msg::text2(user_id, group_id, node_id, &format!("∫group∫ hello {}", 10 * user_id + i), &user_id.to_string());
        io_sender.send(Arc::new(msg)).await?;
    }
    tokio::time::sleep(std::time::Duration::from_secs(100)).await;
    Ok(())
}