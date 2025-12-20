use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File; // Прибрано зайвий `fs` та `self`
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn}; // Прибрано `error`, бо не використовується

// --- Arguments ---
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the snippet to create or read
    #[arg(long)]
    name: Option<String>,

    /// Read a specific snippet
    #[arg(long)]
    read: Option<String>,

    /// Delete a specific snippet
    #[arg(long)]
    delete: Option<String>,

    /// Download snippet content from URL instead of stdin
    #[arg(long)]
    download: Option<String>,
}

// --- Storage Logic ---
#[derive(Serialize, Deserialize, Default, Debug)]
struct Store {
    snippets: HashMap<String, String>,
}

impl Store {
    fn load(path: &Path) -> Result<Self> {
        debug!("Спроба завантаження сховища з {:?}", path);
        if path.exists() {
            let file = File::open(path)
                .with_context(|| format!("Не вдалося відкрити файл сховища: {}", path.display()))?;

            // ВИПРАВЛЕННЯ: Явно вказуємо тип змінної `store: Store`
            let store: Store = serde_json::from_reader(file)
                .context("Не вдалося десеріалізувати файл сховища")?;

            debug!("Сховище успішно завантажено, знайдено {} сніпетів", store.snippets.len());
            Ok(store)
        } else {
            info!("Файл сховища не знайдено, створюємо нове порожнє сховище");
            Ok(Store::default())
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        debug!("Збереження сховища у {:?}", path);
        let file = File::create(path)
            .context("Не вдалося створити файл сховища")?;

        serde_json::to_writer_pretty(file, self)
            .context("Не вдалося записати у файл")?;

        info!("Сховище успішно збережено");
        Ok(())
    }
}

// --- Logging Setup ---
fn init_logging() -> Result<()> {
    let log_level = std::env::var("SNIPPETS_APP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_path = std::env::var("SNIPPETS_APP_LOG_PATH").ok();

    let env_filter = tracing_subscriber::EnvFilter::new(&log_level);

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter);

    if let Some(path_str) = log_path {
        let path = PathBuf::from(path_str);
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("snippets.log"));

        let file_appender = tracing_appender::rolling::never(directory, file_name);

        subscriber
            .with_writer(file_appender)
            .with_ansi(false)
            .init();
    } else {
        subscriber
            .with_writer(std::io::stdout)
            .init();
    }

    Ok(())
}

// --- Main ---
fn main() -> Result<()> {
    init_logging().context("Не вдалося ініціалізувати логування")?;

    let args = Args::parse();
    let store_path = Path::new("snippets.json");

    info!("Запуск snippets-app");

    let mut store = Store::load(store_path)?;

    if let Some(name) = args.name {
        let content = if let Some(url) = args.download {
            info!("Завантаження сніпету з URL: {}", url);
            reqwest::blocking::get(&url)
                .with_context(|| format!("Не вдалося зробити запит до {}", url))?
                .error_for_status()
                .context("Отримано помилку від сервера")?
                .text()
                .context("Не вдалося прочитати текст відповіді")?
        } else {
            debug!("Читання сніпету з stdin");
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .context("Не вдалося прочитати з stdin")?;
            buf
        };

        let content = content.trim().to_string();
        if content.is_empty() {
            warn!("Спроба зберегти порожній сніпет '{}'", name);
        }

        store.snippets.insert(name.clone(), content);
        store.save(store_path)?;

        eprintln!("Сніпет '{}' успішно збережено.", name);
        info!("Сніпет '{}' додано/оновлено.", name);

    } else if let Some(name) = args.read {
        info!("Запит на читання сніпету: {}", name);
        match store.snippets.get(&name) {
            Some(content) => {
                println!("{}", content);
                debug!("Сніпет '{}' виведено в stdout", name);
            },
            None => {
                warn!("Сніпет '{}' не знайдено при спробі читання", name);
                return Err(anyhow!("Сніпет '{}' не знайдено.", name));
            }
        }

    } else if let Some(name) = args.delete {
        info!("Запит на видалення сніпету: {}", name);
        if store.snippets.remove(&name).is_some() {
            store.save(store_path)?;
            eprintln!("Сніпет '{}' видалено.", name);
            info!("Сніпет '{}' успішно видалено з бази", name);
        } else {
            warn!("Сніпет '{}' не знайдено при спробі видалення", name);
            return Err(anyhow!("Сніпет '{}' не знайдено.", name));
        }
    } else {
        use clap::CommandFactory;
        Args::command().print_help().context("Не вдалося вивести довідку")?;
    }

    Ok(())
}