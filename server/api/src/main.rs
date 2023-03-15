use crate::{config::CONFIG, sql::DELETE_AT, util::my_id};

use config::CONFIG_FILE_PATH;
use lib::{joy, Result};
use salvo::{
    cors::Cors,
    hyper::header::HeaderName,
    listener::openssl::{Keycert, OpensslConfig},
    prelude::{empty_handler, OpensslListener},
    Router, Server,
};
use structopt::StructOpt;
use tracing::info;

mod cache;
mod config;
mod error;
mod handler;
mod model;
mod rpc;
mod sql;
mod util;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/message")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./api/config.toml"
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
        .allow_origins(vec![
            "http://localhost:4000",
            "https://localhost:4000",
            "http://localhost:3000",
            "https://localhost:3000",
            "http://localhost:8080",
            "https://localhost:8080",
            "https://localhost",
            "https://127.0.0.1",
        ])
        .allow_credentials(true)
        .allow_headers(vec![
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
        ])
        .build();
    let router = Router::with_hoop(cors)
        .push(
            Router::with_path("/which_node")
                .get(handler::user::which_node)
                .options(empty_handler),
        )
        .push(
            Router::with_path("/which_address")
                .get(handler::user::which_address)
                .options(empty_handler),
        )
        .push(
            Router::with_path("/user")
                .put(handler::user::login)
                .post(handler::user::signup)
                .delete(handler::user::logout)
                .options(empty_handler)
                .push(
                    Router::with_path("/info")
                        .get(handler::user::get_user_info)
                        .put(handler::user::update_user_info)
                        .options(empty_handler),
                )
                .push(
                    Router::with_path("/s-info-r")
                        .get(handler::user::get_remark_avatar)
                        .options(empty_handler),
                )
                .push(
                    Router::with_path("/s-info-n")
                        .get(handler::user::get_nickname_avatar)
                        .options(empty_handler),
                )
                .push(
                    Router::with_path("/account")
                        .delete(handler::user::sign_out)
                        .post(handler::user::new_account_id)
                        .options(empty_handler),
                ),
        )
        .push(
            Router::with_path("/group")
                .post(handler::group::create_group)
                .delete(handler::group::destroy_group)
                .options(empty_handler)
                .push(
                    Router::with_path("/info")
                        .get(handler::group::get_group_info)
                        .put(handler::group::update_group_info)
                        .options(empty_handler)
                        .push(
                            Router::with_path("/member")
                                .get(handler::group::get_group_user_list)
                                .options(empty_handler),
                        ),
                )
                .push(
                    Router::with_path("/user")
                        .post(handler::group::join_group)
                        .delete(handler::group::leave_group)
                        .options(empty_handler)
                        .push(
                            Router::with_path("/admin")
                                .put(handler::group::approve_join)
                                .delete(handler::group::remove_member)
                                .options(empty_handler),
                        ),
                )
                .push(
                    Router::with_path("/admin")
                        .put(handler::group::set_admin)
                        .options(empty_handler),
                ),
        )
        .push(
            Router::with_path("/message")
                .delete(handler::msg::withdraw)
                .put(handler::msg::edit)
                .path("/inbox")
                .get(handler::msg::inbox)
                .options(empty_handler)
                .push(
                    Router::with_path("/unread")
                        .get(handler::msg::unread)
                        .put(handler::msg::update_unread)
                        .options(empty_handler),
                )
                .push(
                    Router::with_path("/history")
                        .get(handler::msg::history_msg)
                        .options(empty_handler),
                ),
        )
        .push(
            Router::with_path("/relationship")
                .post(handler::relationship::add_friend)
                .put(handler::relationship::confirm_add_friend)
                .delete(handler::relationship::delete_friend)
                .get(handler::relationship::get_peer_relationship)
                .options(empty_handler)
                .push(
                    Router::with_path("/friend")
                        .put(handler::relationship::update_relationship)
                        .get(handler::relationship::get_friend_list)
                        .options(empty_handler),
                ),
        )
        .options(empty_handler);
    let key_cert = Keycert::new();
    let key_cert = key_cert.with_cert(CONFIG.server.cert.0.clone());
    let key_cert = key_cert.with_key(CONFIG.server.key.0.clone());
    let listener = OpensslListener::try_with_config(OpensslConfig::new(key_cert))?
        .bind(CONFIG.server.service_address);
    // let listener = TcpListener::bind(CONFIG.server.service_address);
    Server::new(listener).serve(router).await;
    Ok(())
}
