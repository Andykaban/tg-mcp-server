mod libs;

use crate::libs::mcp_server::start_mcp_server_stream;
use crate::libs::tg_client::TgClient;
use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tokio::runtime;

const SESSION_FILE: &str = "tg_mcp_server.session";

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[arg(long)]
    config_path: std::path::PathBuf,
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

async fn async_main(config: TgConfig, bind_address: String) -> Result<()> {
    let tg_client = TgClient::new(
        config.tg_api_id,
        &config.tg_api_hash,
        &config.phone_number,
        SESSION_FILE,
    )
    .await?;
    /*let dialogs = tg_client.get_dialogs().await?;
    for d in dialogs {
        println!(
            "{} -- {} -- {} -- {}",
            d.dialog_id,
            d.dialog_name.unwrap_or("not defined".to_string()),
            d.dialog_full_name.unwrap_or("not defined".to_string()),
            d.dialog_type
        )
    }*/
    _ = start_mcp_server_stream(tg_client, bind_address).await?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let config = read_tg_config_from_file(args.config_path).unwrap();
    let mcp_bind_address = format!("{}:{}", args.mcp_host, args.mcp_port);

    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(config, mcp_bind_address))
}
