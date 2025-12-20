use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, Environment, File};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    debug: bool,

    #[arg(short, long, env = "CONF_FILE", default_value = "config.toml")]
    conf: PathBuf,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AppConfig {
    debug: bool,
    server_addr: String,
    port: u16,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let settings = Config::builder()
        .set_default("debug", false)?
        .set_default("server_addr", "127.0.0.1")?
        .set_default("port", 8080)?
        .add_source(File::from(args.conf).required(false))
        .add_source(Environment::with_prefix("CONF").separator("_"))
        .build()
        .context("Не вдалося зібрати конфігурацію")?;

    match settings.clone().try_deserialize::<AppConfig>() {
        Ok(app_config) => {
            println!("=== Actual Configuration ===");
            println!("{:#?}", app_config);
            
            if args.debug {
                println!("\n(Debug mode enabled via CLI flag)");
            }
        }
        Err(e) => {
            println!("Не вдалося перетворити на структуру AppConfig: {}", e);
            
            let map: std::collections::HashMap<String, String> = settings.try_deserialize().unwrap_or_default();
            println!("Raw Settings: {:#?}", map);
        }
    }

    Ok(())
}