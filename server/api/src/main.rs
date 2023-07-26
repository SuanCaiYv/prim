use crate::{config::CONFIG, sql::DELETE_AT};

use config::CONFIG_FILE_PATH;
use lib::{joy, Result};
use salvo::{
    conn::rustls::{Keycert, RustlsConfig},
    cors::Cors,
    hyper::{header::HeaderName, Method},
    prelude::{QuinnListener, TcpListener},
    Listener, Router, Server,
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
#[structopt(name = "prim/api")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./api/config.toml"
    )]
    pub(crate) config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();
    let config_path = match std::env::var("CONFIG_PATH") {
        Ok(config_path) => config_path,
        Err(_) => opt.config,
    };
    unsafe { CONFIG_FILE_PATH = Box::leak(config_path.into_boxed_str()) }
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
    println!("{}", joy::banner());
    info!("prim api running on {}", CONFIG.server.service_address);
    tokio::spawn(async move {
        if let Err(e) = rpc::start().await {
            tracing::error!("rpc server error: {}", e);
        }
    });
    let cors = Cors::new()
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::OPTIONS,
            Method::DELETE,
            Method::PUT,
            Method::PATCH,
            Method::HEAD,
        ])
        .allow_origin(vec![
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
        .into_handler();
    let router = Router::with_hoop(cors)
        .push(
            Router::with_path("/which_node")
                .get(handler::user::which_node)
                .options(salvo::prelude::handler::empty()),
        )
        .push(
            Router::with_path("/which_address")
                .get(handler::user::which_address)
                .options(salvo::prelude::handler::empty()),
        )
        .push(
            Router::with_path("/new_account_id")
                .get(handler::user::new_account_id)
                .options(salvo::prelude::handler::empty()),
        )
        .push(
            Router::with_path("/user")
                .push(
                    Router::new()
                        .put(handler::user::login)
                        .options(salvo::prelude::handler::empty())
                        .post(handler::user::signup)
                        .options(salvo::prelude::handler::empty())
                        .delete(handler::user::logout)
                        .options(salvo::prelude::handler::empty()),
                )
                .push(
                    Router::with_path("/info")
                        .get(handler::user::get_user_info)
                        .put(handler::user::update_user_info)
                        .options(salvo::prelude::handler::empty()),
                )
                .push(
                    Router::with_path("/s-info-r")
                        .get(handler::user::get_remark_avatar)
                        .options(salvo::prelude::handler::empty()),
                )
                .push(
                    Router::with_path("/s-info-n")
                        .get(handler::user::get_nickname_avatar)
                        .options(salvo::prelude::handler::empty()),
                )
                .push(
                    Router::with_path("/account")
                        .delete(handler::user::sign_out)
                        .post(handler::user::new_account_id)
                        .options(salvo::prelude::handler::empty()),
                ),
        )
        .push(
            Router::with_path("/group")
                .post(handler::group::create_group)
                .delete(handler::group::destroy_group)
                .options(salvo::prelude::handler::empty())
                .push(
                    Router::with_path("/info")
                        .get(handler::group::get_group_info)
                        .put(handler::group::update_group_info)
                        .options(salvo::prelude::handler::empty())
                        .push(
                            Router::with_path("/member")
                                .get(handler::group::get_group_user_list)
                                .options(salvo::prelude::handler::empty()),
                        ),
                )
                .push(
                    Router::with_path("/user")
                        .push(
                            Router::new()
                                .post(handler::group::join_group)
                                .delete(handler::group::leave_group)
                                .options(salvo::prelude::handler::empty()),
                        )
                        .push(
                            Router::with_path("/admin")
                                .put(handler::group::approve_join)
                                .delete(handler::group::remove_member)
                                .options(salvo::prelude::handler::empty()),
                        ),
                )
                .push(
                    Router::new()
                        .put(handler::group::set_admin)
                        .options(salvo::prelude::handler::empty()),
                ),
        )
        .push(
            Router::with_path("/message")
                .push(
                    Router::new()
                        .delete(handler::msg::withdraw)
                        .put(handler::msg::edit)
                        .path("/inbox")
                        .get(handler::msg::inbox)
                        .options(salvo::prelude::handler::empty()),
                )
                .push(
                    Router::with_path("/unread")
                        .get(handler::msg::unread)
                        .put(handler::msg::update_unread)
                        .options(salvo::prelude::handler::empty()),
                )
                .push(
                    Router::with_path("/history")
                        .get(handler::msg::history_msg)
                        .options(salvo::prelude::handler::empty()),
                ),
        )
        .push(
            Router::with_path("/relationship")
                .push(
                    Router::new()
                        .post(handler::relationship::add_friend)
                        .put(handler::relationship::confirm_add_friend)
                        .delete(handler::relationship::delete_friend)
                        .get(handler::relationship::get_peer_relationship)
                        .options(salvo::prelude::handler::empty()),
                )
                .push(
                    Router::with_path("/friend")
                        .put(handler::relationship::update_relationship)
                        .get(handler::relationship::get_friend_list)
                        .options(salvo::prelude::handler::empty()),
                ),
        )
        .options(salvo::prelude::handler::empty());
    let config = RustlsConfig::new(
        Keycert::new()
            .cert(CONFIG.server.cert.0.clone())
            .key(CONFIG.server.key.0.clone()),
    );
    let mut version1_address = CONFIG.server.service_address.clone();
    version1_address.set_port(version1_address.port() + 2);
    let listener = TcpListener::new(version1_address);
    let acceptor = TcpListener::new(CONFIG.server.service_address).rustls(config.clone());
    let acceptor = QuinnListener::new(config, CONFIG.server.service_address)
        .join(acceptor)
        .join(listener)
        .bind()
        .await;
    Server::new(acceptor).serve(router).await;
    Ok(())
}
