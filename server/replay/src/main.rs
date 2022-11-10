use common::joy;
use common::Result;
use config::{CONFIG, CONFIG_FILE_PATH};
use structopt::StructOpt;

mod config;
pub mod core;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/replay")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./config.toml"
    )]
    pub(crate) config: String,
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
    println!("{}", joy::banner());
    core::start().await?;
    Ok(())
}
