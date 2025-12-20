use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// Import Layer and prelude for better logging setup (виправлення)
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt, registry};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {

    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    read: Option<String>,

    #[arg(long)]
    delete: Option<String>,

    #[arg(long)]
    download: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Store {
    pub snippets: HashMap<String, String>,
}

impl Store {
    pub fn load(path: &Path) -> Result<Self> {
        debug!("Спроба завантаження сховища з {:?}", path);
        if path.exists() {
            let file = File::open(path)
                .with_context(|| format!("Не вдалося відкрити файл сховища: {}", path.display()))?;

            let store: Store = serde_json::from_reader(file)
                .context("Не вдалося десеріалізувати файл сховища. Переконайтеся, що файл JSON валідний")?;

            debug!("Сховище успішно завантажено, знайдено {} сніпетів", store.snippets.len());
            Ok(store)
        } else {
            info!("Файл сховища не знайдено, створюємо нове порожнє сховище");
            Ok(Store::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        debug!("Збереження сховища у {:?}", path);
        let file = File::create(path)
            .context("Не вдалося створити файл сховища")?;

        serde_json::to_writer_pretty(file, self)
            .context("Не вдалося записати у файл")?;

        info!("Сховище успішно збережено");
        Ok(())
    }
}

fn init_logging() -> Result<()> {
    let log_level = std::env::var("SNIPPETS_APP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_path = std::env::var("SNIPPETS_APP_LOG_PATH").ok();

    let env_filter = EnvFilter::new(&log_level);
    let subscriber = registry().with(env_filter);

    if let Some(path_str) = log_path {
        let path = PathBuf::from(&path_str);
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("snippets.log"));

        let file_appender = tracing_appender::rolling::never(directory, file_name);

        let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking_writer);

        subscriber
            .with(file_layer)
            .try_init()
            .context("Не вдалося встановити логування у файл")?;

    } else {
        let console_layer = fmt::layer()
            .pretty()
            .with_writer(std::io::stderr);

        subscriber
            .with(console_layer)
            .try_init()
            .context("Не вдалося встановити консольне логування")?;
    }

    Ok(())
}

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
            warn!("Спроба зберегти порожній сніпет '{}'. Сніпет буде збережено як порожній рядок.", name);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_temp_json_file(content: Option<&str>) -> NamedTempFile {
        let temp_file = NamedTempFile::new().expect("Failed to create temporary file");
        if let Some(c) = content {
            std::fs::write(temp_file.path(), c).expect("Failed to write to temporary file");
        } else {
            std::fs::write(temp_file.path(), r#"{"snippets": {}}"#).expect("Failed to write empty JSON");
        }
        temp_file
    }

    #[test]
    fn store_load_non_existent_file_returns_default() -> Result<()> {
        let path = Path::new("non_existent_file_for_test.json");
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let store = Store::load(path)?;
        assert!(store.snippets.is_empty(), "Сховище має бути порожнім, якщо файл не існує");
        Ok(())
    }

    #[test]
    fn store_load_valid_file() -> Result<()> {
        let content = r#"{"snippets": {"snippet_a": "content_a", "snippet_b": "content_b"}}"#;
        let temp_file = setup_temp_json_file(Some(content));

        let store = Store::load(temp_file.path())?;

        assert_eq!(store.snippets.len(), 2, "Має бути 2 сніпети");
        assert_eq!(store.snippets.get("snippet_a"), Some(&"content_a".to_string()));
        Ok(())
    }

    #[test]
    fn store_load_empty_or_invalid_json_returns_error() {
        let invalid_content = r#"{"snippets": "invalid"}"#;
        let temp_file_invalid = setup_temp_json_file(Some(invalid_content));
        assert!(Store::load(temp_file_invalid.path()).is_err(), "Має бути помилка для невалідного JSON");

        let broken_content = r#"not json"#;
        let temp_file_broken = setup_temp_json_file(Some(broken_content));
        assert!(Store::load(temp_file_broken.path()).is_err(), "Має бути помилка для пошкодженого JSON");
    }

    #[test]
    fn store_save_file() -> Result<()> {
        let temp_file = setup_temp_json_file(None);
        let path = temp_file.path();

        let mut store = Store::default();
        store.snippets.insert("test_key".to_string(), "test_value".to_string());

        store.save(path)?;

        let saved_content = std::fs::read_to_string(path)?;

        assert!(saved_content.contains("test_key"));
        assert!(saved_content.contains("test_value"));

        let loaded_store = Store::load(path)?;
        assert_eq!(loaded_store.snippets.len(), 1, "Має бути 1 сніпет після збереження/завантаження");
        assert_eq!(loaded_store.snippets.get("test_key"), Some(&"test_value".to_string()));

        Ok(())
    }

    #[test]
    fn store_save_and_overwrite() -> Result<()> {
        let content = r#"{"snippets": {"old_key": "old_value"}}"#;
        let temp_file = setup_temp_json_file(Some(content));
        let path = temp_file.path();

        let mut store = Store::load(path)?;

        store.snippets.insert("new_key".to_string(), "new_value".to_string());
        store.snippets.remove("old_key");

        store.save(path)?;

        let loaded_store = Store::load(path)?;
        assert_eq!(loaded_store.snippets.len(), 1, "Має бути 1 сніпет після перезапису");
        assert!(loaded_store.snippets.contains_key("new_key"), "Новий ключ має бути присутній");
        assert!(!loaded_store.snippets.contains_key("old_key"), "Старий ключ має бути видалений");

        Ok(())
    }
}