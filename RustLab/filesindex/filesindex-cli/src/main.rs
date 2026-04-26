#![warn(clippy::missing_errors_doc, clippy::result_large_err)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use filesindex_core::{FileEntry, JsonStorage, SqliteStorage, Storage};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "filesindex")]
#[command(about = "File indexing and classification utility")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Add {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        tags: String,
    },
    Get {
        #[arg(long)]
        tags: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let env_val = std::env::var("FILES_INDEX_PATH")
        .unwrap_or_else(|_| "json:./files_index.json".to_string());

    let mut parts = env_val.splitn(2, ':');
    let storage_type = parts.next().unwrap_or("json");
    let storage_path = parts.next().unwrap_or("./files_index.json");

    let mut storage: Box<dyn Storage> = match storage_type {
        "sqlite" => Box::new(
            SqliteStorage::new(PathBuf::from(storage_path))
                .context("Failed to open SQLite storage")?
        ),
        _ => Box::new(
            JsonStorage::new(PathBuf::from(storage_path))
                .context("Failed to open JSON storage")?
        ),
    };

    match cli.command {
        Command::Add { path, tags } => {
            let tags: Vec<String> = tags.split(',').map(String::from).collect();
            let entry = FileEntry {
                path: path.to_string_lossy().to_string(),
                tags,
            };
            storage.add(entry).context("Failed to add entry")?;
            println!("Added: {}", path.display());
        }
        Command::Get { tags } => {
            let tags: Vec<String> = tags.split(',').map(String::from).collect();
            let results = storage.get_by_tags(&tags).context("Failed to get entries")?;
            if results.is_empty() {
                println!("No files found");
            } else {
                for entry in results {
                    println!("{} [{}]", entry.path, entry.tags.join(", "));
                }
            }
        }
    }

    Ok(())
}
