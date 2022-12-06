use crate::{config::CONFIG, util::my_id};
use config::CONFIG_FILE_PATH;
use lib::{joy, Result};
use salvo::{
    cors::Cors,
    hyper::header::HeaderName,
    prelude::{empty_handler, TcpListener},
    Router, Server,
};
use structopt::StructOpt;
use tracing::info;

mod cache;
mod config;
mod entity;
mod handler;
mod rpc;
mod sql;
mod util;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/message")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./message/config.toml"
    )]
    pub(crate) config: String,
    #[structopt(
        long = "my_id",
        long_help = r"manually set 'my_id' of server node",
        default_value = "0"
    )]
    pub(crate) my_id: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();
    unsafe { CONFIG_FILE_PATH = Box::leak(opt.config.into_boxed_str()) }
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(CONFIG.log_level)
        .try_init()
        .unwrap();
    util::load_my_id(opt.my_id).await?;
    // rpc::gen();
    println!("{}", joy::banner());
    info!(
        "prim api[{}] running on {}",
        my_id(),
        CONFIG.server.service_address
    );
    tokio::spawn(async move {
        if let Err(e) = rpc::start().await {
            tracing::error!("rpc server error: {}", e);
        }
    });
    let cors = Cors::builder()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS", "HEAD"])
        .allow_headers(vec![
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
        ])
        .build();
    let router = Router::with_hoop(cors)
        .options(empty_handler)
        .push(
            Router::with_path("/user")
                .path("/user")
                .put(handler::user::login)
                .post(handler::user::signup)
                .delete(handler::user::logout),
        )
        .push(
            Router::with_path("/user/account")
                .delete(handler::user::sign_out)
                .post(handler::user::new_account_id),
        )
        .push(Router::with_path("/which_node/<user_id>").get(handler::user::which_node));
    Server::new(TcpListener::bind(CONFIG.server.service_address))
        .serve(router)
        .await;
    Ok(())
}
