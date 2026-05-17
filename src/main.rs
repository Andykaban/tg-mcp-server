mod libs;

use crate::libs::mcp_server::{start_mcp_server_http, start_mcp_server_stdio};
use crate::libs::tg_client::TgClient;
use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tokio::runtime;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const SESSION_FILE: &str = "tg_mcp_server.session";

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[arg(long)]
    config_path: std::path::PathBuf,
    #[arg(long)]
    transport: String,
    #[arg(long, default_value = "127.0.0.1")]
    mcp_host: String,
    #[arg(long, default_value = "9050")]
    mcp_port: String,
}

#[derive(Deserialize, Debug)]
struct TgConfig {
    tg_api_id: i32,
    tg_api_hash: String,
    phone_number: String,
}

fn read_tg_config_from_file<P: AsRef<Path>>(path: P) -> Result<TgConfig, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}

async fn async_main(config: TgConfig, transport: String, bind_address: String) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();
    let tg_client = TgClient::new(
        config.tg_api_id,
        &config.tg_api_hash,
        &config.phone_number,
        SESSION_FILE,
    )
    .await?;
    match transport.as_str() {
        "stdio" => start_mcp_server_stdio(tg_client).await?,
        "http" => start_mcp_server_http(tg_client, bind_address).await?,
        &_ => panic!(
            "unsupported transport value: {} (expected 'stdio' or 'http')",
            transport
        ),
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let config = read_tg_config_from_file(args.config_path).unwrap();
    let transport = args.transport;
    let mcp_bind_address = format!("{}:{}", args.mcp_host, args.mcp_port);

    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(config, transport, mcp_bind_address))
}
