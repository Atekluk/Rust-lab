//! Прикладна програма для керування текстовими сніпетами.
//!
//! Дозволяє зберігати, читати та видаляти невеликі текстові фрагменти
//! з локального JSON-файлу. Підтримує читання з stdin або завантаження з URL.
#![warn(
    missing_docs,
    broken_intra_doc_links,
    missing_crate_level_docs,
    unreachable_pub
)]
#![warn(clippy::missing_panics_doc, clippy::clone_on_ref_ptr, clippy::similar_names)]

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use quote::quote;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use syn::{
    parse::{Parse, ParseStream, Result as SynResult},
    parse_macro_input, Expr, Token,
};
use tracing::{debug, info, warn};
use tracing_appender::non_blocking;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, registry, EnvFilter};

// --- Макроси ---

/// Створює BTreeMap декларативним способом (аналогічно до vec!).
///
/// # Приклади
///
/// ```
/// use std::collections::BTreeMap;
/// // Створення порожньої мапи
/// let map_empty: BTreeMap<i32, &str> = btreemap_decl!();
/// assert!(map_empty.is_empty());
///
/// // Створення мапи з елементами
/// let map = btreemap_decl!(1 => "a", 2 => "b",);
/// assert_eq!(*map.get(&1).unwrap(), "a");
/// ```
#[macro_export]
macro_rules! btreemap_decl {
    // Правило 1: Порожня мапа
    () => {
        std::collections::BTreeMap::new()
    };

    // Правило 2: Мапа з елементами
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            let mut map = std::collections::BTreeMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }
    };
}

// УВАГА: Наступний код для процедурного макросу має бути розміщений
// в окремому крейті з `proc-macro = true` у Cargo.toml.
// Тут він наведено лише для повноти демонстрації логіки.

struct KeyValue {
    key: Expr,
    value: Expr,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let key = input.parse()?;
        input.parse::<Token![=>]>()?;
        let value = input.parse()?;
        Ok(KeyValue { key, value })
    }
}

struct MapArgs {
    pairs: Vec<KeyValue>,
}

impl Parse for MapArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut pairs = Vec::new();
        while !input.is_empty() {
            pairs.push(input.parse()?);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(MapArgs { pairs })
    }
}

/// Створює BTreeMap за допомогою Процедурного макросу.
///
/// Ця функція лише демонструє логіку, яка має бути у крейті процедурних макросів.
///
/// # Errors
///
/// Повертає помилку, якщо вхідний синтаксис не є валідним списком `Key => Value` пар.
pub fn btreemap_proc_logic(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let MapArgs { pairs } = match syn::parse2(input) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error(),
    };

    let inserts = pairs.into_iter().map(|kv| {
        let key = kv.key;
        let value = kv.value;
        quote! {
            map.insert(#key, #value);
        }
    });

    quote! {
        {
            let mut map = std::collections::BTreeMap::new();
            #(#inserts)*
            map
        }
    }
}

// --- Arguments ---
/// Управління сніпетами: збереження, читання та видалення текстових фрагментів.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Ім'я сніпету, який потрібно створити або оновити.
    #[arg(long)]
    name: Option<String>,

    /// Прочитати вміст вказаного сніпету та вивести на stdout.
    #[arg(long)]
    read: Option<String>,

    /// Видалити вказаний сніпет.
    #[arg(long)]
    delete: Option<String>,

    /// Завантажити вміст сніпету з URL замість stdin.
    #[arg(long)]
    download: Option<String>,
}

// --- Storage Logic ---
/// Структура для зберігання всіх сніпетів.
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Store {
    /// Карта, де ключ — це ім'я сніпету, а значення — його вміст.
    pub snippets: HashMap<String, String>,
}

impl Store {
    /// Завантажує дані сховища з вказаного шляху.
    ///
    /// Якщо файл не знайдено, повертає нове, порожнє сховище.
    ///
    /// # Errors
    ///
    /// Повертає помилку, якщо:
    /// * Не вдалося відкрити файл (через проблеми з доступом або шляхом).
    /// * Файл містить невалідний JSON або некоректну структуру сховища.
    pub fn load(path: &Path) -> Result<Self> {
        debug!("Спроба завантаження сховища з {:?}", path);
        if path.exists() {
            let file = File::open(path).with_context(|| {
                format!("Не вдалося відкрити файл сховища: {}", path.display())
            })?;

            let store: Store = serde_json::from_reader(file).context(
                "Не вдалося десеріалізувати файл сховища. Переконайтеся, що файл JSON валідний",
            )?;

            debug!(
                "Сховище успішно завантажено, знайдено {} сніпетів",
                store.snippets.len()
            );
            Ok(store)
        } else {
            info!("Файл сховища не знайдено, створюємо нове порожнє сховище");
            Ok(Store::default())
        }
    }

    /// Зберігає поточний стан сховища у вказаний шлях.
    ///
    /// # Errors
    ///
    /// Повертає помилку, якщо:
    /// * Не вдалося створити або відкрити файл для запису.
    /// * Не вдалося серіалізувати дані у формат JSON.
    pub fn save(&self, path: &Path) -> Result<()> {
        debug!("Збереження сховища у {:?}", path);
        let file = File::create(path).context("Не вдалося створити файл сховища")?;

        serde_json::to_writer_pretty(file, self).context("Не вдалося записати у файл")?;

        info!("Сховище успішно збережено");
        Ok(())
    }
}

// --- Logging Setup ---
/// Ініціалізує глобальний логувальник на основі змінних середовища.
///
/// Використовує змінні `SNIPPETS_APP_LOG_LEVEL` (за замовчуванням "info")
/// та `SNIPPETS_APP_LOG_PATH`.
///
/// Якщо `SNIPPETS_APP_LOG_PATH` встановлено, логи йдуть у файл (без ANSI).
/// Інакше логи йдуть у `stderr` з кольоровим форматуванням.
///
/// # Errors
///
/// Повертає помилку, якщо логувальник не вдалося ініціалізувати (наприклад,
/// він вже був ініціалізований).
fn init_logging() -> Result<()> {
    let log_level =
        std::env::var("SNIPPETS_APP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_path = std::env::var("SNIPPETS_APP_LOG_PATH").ok();

    let env_filter = EnvFilter::new(&log_level);
    let subscriber = registry().with(env_filter);

    if let Some(path_str) = log_path {
        let path = PathBuf::from(&path_str);
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("snippets.log"));

        let file_appender = tracing_appender::rolling::never(directory, file_name);

        // Використання non_blocking для запобігання блокуванню головного потоку
        let (non_blocking_writer, _guard) = non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_ansi(false) // No ANSI codes in log files
            .with_writer(non_blocking_writer);

        subscriber
            .with(file_layer)
            .try_init()
            .context("Не вдалося встановити логування у файл")?;
    } else {
        let console_layer = fmt::layer()
            .pretty()
            .with_writer(std::io::stderr); // Логи мають іти у stderr

        subscriber
            .with(console_layer)
            .try_init()
            .context("Не вдалося встановити консольне логування")?;
    }

    Ok(())
}

// --- Main ---
/// Головна функція застосунку.
///
/// # Errors
///
/// Повертає помилку, якщо:
/// * Не вдалося ініціалізувати логування.
/// * Не вдалося завантажити/зберегти сховище.
/// * Помилка вводу/виводу при читанні з stdin.
/// * Помилка мережі при завантаженні з URL.
/// * Запитано сніпет, який не існує.
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
            warn!(
                "Спроба зберегти порожній сніпет '{}'. Сніпет буде збережено як порожній рядок.",
                name
            );
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
            }
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
        Args::command()
            .print_help()
            .context("Не вдалося вивести довідку")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use tempfile::NamedTempFile;

    // Допоміжна функція для створення тимчасового файлу JSON
    fn setup_temp_json_file(content: Option<&str>) -> NamedTempFile {
        let temp_file = NamedTempFile::new().expect("Failed to create temporary file");
        if let Some(c) = content {
            std::fs::write(temp_file.path(), c).expect("Failed to write to temporary file");
        } else {
            // Записуємо валідний порожній JSON об'єкт за замовчуванням
            std::fs::write(temp_file.path(), r#"{"snippets": {}}"#)
                .expect("Failed to write empty JSON");
        }
        temp_file
    }

    // --- Тести Store Logic ---

    #[test]
    fn store_load_non_existent_file_returns_default() -> Result<()> {
        let path = Path::new("non_existent_file_for_test.json");
        // Переконаємося, що файл не існує
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let store = Store::load(path)?;
        assert!(
            store.snippets.is_empty(),
            "Сховище має бути порожнім, якщо файл не існує"
        );
        Ok(())
    }

    #[test]
    fn store_load_valid_file() -> Result<()> {
        let content = r#"{"snippets": {"snippet_a": "content_a", "snippet_b": "content_b"}}"#;
        let temp_file = setup_temp_json_file(Some(content));

        let store = Store::load(temp_file.path())?;

        assert_eq!(store.snippets.len(), 2, "Має бути 2 сніпети");
        assert_eq!(
            store.snippets.get("snippet_a"),
            Some(&"content_a".to_string())
        );
        Ok(())
    }

    #[test]
    fn store_load_empty_or_invalid_json_returns_error() {
        // Тест на невалідний JSON
        let invalid_content = r#"{"snippets": "invalid"}"#;
        let temp_file_invalid = setup_temp_json_file(Some(invalid_content));
        // Очікуємо помилку десеріалізації
        assert!(
            Store::load(temp_file_invalid.path()).is_err(),
            "Має бути помилка для невалідного JSON"
        );

        // Тест на повністю пошкоджений JSON
        let broken_content = r#"not json"#;
        let temp_file_broken = setup_temp_json_file(Some(broken_content));
        assert!(
            Store::load(temp_file_broken.path()).is_err(),
            "Має бути помилка для пошкодженого JSON"
        );
    }

    #[test]
    fn store_save_file() -> Result<()> {
        let temp_file = setup_temp_json_file(None);
        let path = temp_file.path();

        let mut store = Store::default();
        store
            .snippets
            .insert("test_key".to_string(), "test_value".to_string());

        store.save(path)?;

        // Зчитуємо файл назад для перевірки
        let saved_content = std::fs::read_to_string(path)?;

        // Перевіряємо вміст
        assert!(saved_content.contains("test_key"));
        assert!(saved_content.contains("test_value"));

        // Перевіряємо, чи можна успішно перезавантажити збережене
        let loaded_store = Store::load(path)?;
        assert_eq!(
            loaded_store.snippets.len(),
            1,
            "Має бути 1 сніпет після збереження/завантаження"
        );
        assert_eq!(
            loaded_store.snippets.get("test_key"),
            Some(&"test_value".to_string())
        );

        Ok(())
    }

    // --- Тести Макросів ---

    #[test]
    fn decl_macro_empty() {
        let map: BTreeMap<i32, &str> = btreemap_decl!();
        assert!(map.is_empty());
    }

    #[test]
    fn decl_macro_multiple_elements() {
        let map = btreemap_decl!(3 => "three", 1 => "one", 2 => "two");
        let mut expected = BTreeMap::new();
        expected.insert(1, "one");
        expected.insert(2, "two");
        expected.insert(3, "three");

        assert_eq!(map.len(), 3);
        assert_eq!(map, expected);
        assert_eq!(map.keys().collect::<Vec<_>>(), vec![&1, &2, &3]);
    }

    #[test]
    fn decl_macro_trailing_comma() {
        let map = btreemap_decl!("a" => 10, "b" => 20,);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("a"), Some(&10));
    }

    #[test]
    fn proc_macro_logic_single_element() {
        let tokens: TokenStream = quote! { 1 => "one" };
        let generated = btreemap_proc_logic(tokens);

        // Перевірка, що згенерований код компілюється та виконується
        let map: BTreeMap<i32, &str> = syn::parse2(generated).unwrap();
        let mut expected = BTreeMap::new();
        expected.insert(1, "one");
        assert_eq!(map, expected);
    }

    #[test]
    fn proc_macro_logic_expressions() {
        let key_expr = 2 + 2;
        let tokens: TokenStream = quote! {
            #key_expr => 42 / 2,
            5 => 100
        };
        let generated = btreemap_proc_logic(tokens);

        // Перевірка, що згенерований код обробляє вирази
        let map: BTreeMap<i32, i32> = syn::parse2(generated).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&4), Some(&21));
        assert_eq!(map.get(&5), Some(&100));
    }
}