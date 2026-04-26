#![warn(
    missing_docs,
    broken_intra_doc_links,
    missing_crate_level_docs,
    unreachable_pub
)]
#![warn(
    clippy::missing_panics_doc,
    clippy::clone_on_ref_ptr,
    clippy::similar_names
)]
//! # snippets-app
//!
//! CLI-утиліта для роботи з кодовими сніпетами.
//!
//! Можливості:
//! - створення та оновлення сніпетів (`--name`, stdin або `--download <URL>`);
//! - читання сніпету за ім'ям (`--read <NAME>`);
//! - видалення сніпету (`--delete <NAME>`);
//! - логування у файл або stdout через `SNIPPETS_APP_LOG_LEVEL` і `SNIPPETS_APP_LOG_PATH`.

use anyhow::{bail, Context, Result};
use clap::Parser;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
};
use tracing::{debug, info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

/// CLI-параметри для утиліти `snippets-app`.
#[derive(Parser, Debug)]
#[command(name = "snippets-app")]
#[command(about = "Create, read and delete code snippets", long_about = None)]
struct Cli {
    /// Ім'я сніпета для створення або оновлення.
    #[arg(long)]
    name: Option<String>,

    /// Прочитати сніпет за ім'ям.
    #[arg(long)]
    read: Option<String>,

    /// Видалити сніпет за ім'ям.
    #[arg(long)]
    delete: Option<String>,

    /// Завантажити вміст сніпета з URL замість читання зі stdin.
    #[arg(long)]
    download: Option<String>,
}

/// Просте сховище сніпетів.
///
/// Ключ — ім'я сніпета, значення — текст коду.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    snippets: HashMap<String, String>,
}

/// Обчислює шлях до JSON-файла сховища.
///
/// Якщо задано змінну оточення `SNIPPETS_APP_STORE`,
/// використовується саме вона, інакше:
/// `~/.snippets-app/snippets.json`.
fn store_path() -> PathBuf {
    if let Ok(p) = std::env::var("SNIPPETS_APP_STORE") {
        return PathBuf::from(p);
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".snippets-app")
        .join("snippets.json")
}

/// Завантажує сховище сніпетів з диска.
///
/// Якщо файл не існує, повертає порожнє сховище.
fn load_store(path: &Path) -> Result<Store> {
    if !path.exists() {
        return Ok(Store::default());
    }
    let file = File::open(path).with_context(|| format!("reading store {}", path.display()))?;
    let store: Store = serde_json::from_reader(file)
        .with_context(|| format!("parsing store {}", path.display()))?;
    Ok(store)
}

/// Зберігає сховище сніпетів у JSON-файл.
fn save_store(path: &Path, store: &Store) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let file = File::create(path).with_context(|| format!("writing store {}", path.display()))?;
    serde_json::to_writer_pretty(file, store)
        .with_context(|| format!("serializing store {}", path.display()))?;
    Ok(())
}

/// Ініціалізує логування з урахуванням змінної `SNIPPETS_APP_LOG_LEVEL`.
fn init_tracing_from_env() {
    let level = std::env::var("SNIPPETS_APP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    let filter = EnvFilter::try_new(&level).unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

/// Додає запис у лог-файл, якщо встановлено `SNIPPETS_APP_LOG_PATH`.
fn log_to_file(level: &str, msg: &str) {
    if let Ok(path) = std::env::var("SNIPPETS_APP_LOG_PATH") {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(PathBuf::from(path))
        {
            use std::io::Write;
            let _ = writeln!(file, "[{level}] {msg}");
        }
    }
}

/// Читає вміст сніпета зі стандартного вводу.
///
/// Повертає помилку, якщо вхід порожній.
fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let trimmed = buf.trim().to_string();
    if trimmed.is_empty() {
        bail!("stdin is empty, nothing to save");
    }
    Ok(trimmed)
}

/// Завантажує сніпет з вказаного URL.
///
/// Повертає помилку, якщо статус HTTP неуспішний
/// або тіло відповіді порожнє.
fn download_snippet(url: &str) -> Result<String> {
    let resp =
        reqwest::blocking::get(url).with_context(|| format!("downloading snippet from {url}"))?;
    if !resp.status().is_success() {
        bail!("download failed with status {}", resp.status());
    }
    let body = resp
        .text()
        .with_context(|| format!("reading response body from {url}"))?;
    let trimmed = body.trim().to_string();
    if trimmed.is_empty() {
        bail!("downloaded snippet is empty");
    }
    Ok(trimmed)
}

/// Точка входу до застосунку `snippets-app`.
///
/// Опрацьовує CLI-прапорці, виконує одну дію:
/// створення/оновлення, читання або видалення сніпета.
fn main() -> Result<()> {
    init_tracing_from_env();

    let args = Cli::parse();
    debug!("Parsed CLI args: {:?}", args);

    // рівно одна дія: name АБО read АБО delete
    let actions =
        args.name.is_some() as u8 + args.read.is_some() as u8 + args.delete.is_some() as u8;
    if actions != 1 {
        bail!("Please specify exactly one of: --name OR --read OR --delete");
    }

    // --download можна тільки з --name
    if args.download.is_some() && args.name.is_none() {
        bail!("--download can only be used together with --name");
    }

    let store_path = store_path();
    let mut store = load_store(&store_path)?;

    if let Some(name) = args.name {
        let content = if let Some(url) = args.download {
            info!("Downloading snippet `{name}` from URL: {url}");
            log_to_file("INFO", &format!("Downloading `{name}` from {url}"));
            download_snippet(&url)?
        } else {
            info!("Reading snippet `{name}` from stdin");
            log_to_file("INFO", &format!("Reading `{name}` from stdin"));
            read_stdin()?
        };

        store.snippets.insert(name.clone(), content);
        save_store(&store_path, &store)?;
        info!("Saved snippet `{name}`");
        log_to_file("INFO", &format!("Saved snippet `{name}`"));
        println!("OK: saved snippet {name}");
        return Ok(());
    }

    if let Some(name) = args.read {
        match store.snippets.get(&name) {
            Some(code) => {
                info!("Reading snippet `{name}`");
                log_to_file("INFO", &format!("Read snippet `{name}`"));
                println!("{code}");
            }
            None => {
                warn!("Snippet `{name}` not found");
                log_to_file("WARN", &format!("Snippet `{name}` not found"));
                bail!("Snippet not found: {name}");
            }
        }
        return Ok(());
    }

    if let Some(name) = args.delete {
        if store.snippets.remove(&name).is_some() {
            save_store(&store_path, &store)?;
            info!("Deleted snippet `{name}`");
            log_to_file("INFO", &format!("Deleted snippet `{name}`"));
            println!("OK: deleted {name}");
        } else {
            warn!("Snippet `{name}` not found for delete");
            log_to_file("WARN", &format!("Snippet `{name}` not found for delete"));
            bail!("Snippet not found: {name}");
        }
        return Ok(());
    }

    Ok(())
}
