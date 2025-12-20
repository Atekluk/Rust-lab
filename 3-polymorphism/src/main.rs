use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ==========================================
// ЧАСТИНА 1: Поліморфізм (Статичний та Динамічний)
// ==========================================

trait Storage<K, V> {
    fn set(&mut self, key: K, val: V);
    fn get(&self, key: &K) -> Option<&V>;
    fn remove(&mut self, key: &K) -> Option<V>;
}

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    email: Cow<'static, str>,
    activated: bool,
}

// --- Реалізація зі статичною диспетчеризацією ---
struct UserRepositoryStatic<S> {
    storage: S,
}

impl<S: Storage<u64, User>> UserRepositoryStatic<S> {
    fn new(storage: S) -> Self {
        Self { storage }
    }

    fn add(&mut self, user: User) {
        self.storage.set(user.id, user);
    }

    fn get(&self, id: u64) -> Option<&User> {
        self.storage.get(&id)
    }

    fn update(&mut self, user: User) {
        self.storage.set(user.id, user);
    }

    fn remove(&mut self, id: u64) -> Option<User> {
        self.storage.remove(&id)
    }
}

// --- Реалізація з динамічною диспетчеризацією ---
struct UserRepositoryDynamic {
    storage: Box<dyn Storage<u64, User>>,
}

impl UserRepositoryDynamic {
    fn new(storage: Box<dyn Storage<u64, User>>) -> Self {
        Self { storage }
    }

    fn add(&mut self, user: User) {
        self.storage.set(user.id, user);
    }

    fn get(&self, id: u64) -> Option<&User> {
        self.storage.get(&id)
    }

    fn update(&mut self, user: User) {
        self.storage.set(user.id, user);
    }

    fn remove(&mut self, id: u64) -> Option<User> {
        self.storage.remove(&id)
    }
}

// --- Mock Storage для тестів ---
struct InMemoryStorage<K, V> {
    map: HashMap<K, V>,
}

impl<K, V> InMemoryStorage<K, V> {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> Storage<K, V> for InMemoryStorage<K, V> {
    fn set(&mut self, key: K, val: V) {
        self.map.insert(key, val);
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.map.remove(key)
    }
}

// ==========================================
// ЧАСТИНА 2: Додаток Snippets (JSON & SQLite)
// ==========================================

#[derive(Debug, Serialize, Deserialize)]
struct Snippet {
    content: String,
    created_at: DateTime<Utc>,
}

trait SnippetStorage {
    fn add(&mut self, snippet: Snippet) -> Result<(), String>;
    fn list_all(&self) -> Result<Vec<Snippet>, String>;
}

// --- Реалізація JSON ---
struct JsonStorage {
    file_path: String,
}

impl SnippetStorage for JsonStorage {
    fn add(&mut self, snippet: Snippet) -> Result<(), String> {
        let mut snippets = self.list_all().unwrap_or_default();
        snippets.push(snippet);

        let file = File::create(&self.file_path).map_err(|e| e.to_string())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &snippets).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<Snippet>, String> {
        if !Path::new(&self.file_path).exists() {
            return Ok(vec![]);
        }
        let file = File::open(&self.file_path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let snippets: Vec<Snippet> = serde_json::from_reader(reader).unwrap_or_else(|_| vec![]);
        Ok(snippets)
    }
}

// --- Реалізація SQLite ---
struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    fn new(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snippets (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
            .map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }
}

impl SnippetStorage for SqliteStorage {
    fn add(&mut self, snippet: Snippet) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO snippets (content, created_at) VALUES (?1, ?2)",
                params![snippet.content, snippet.created_at.to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<Snippet>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT content, created_at FROM snippets")
            .map_err(|e| e.to_string())?;

        let snippet_iter = stmt
            .query_map([], |row| {
                let content: String = row.get(0)?;
                let created_at_str: String = row.get(1)?;
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?;

                Ok(Snippet {
                    content,
                    created_at,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut snippets = Vec::new();
        for s in snippet_iter {
            snippets.push(s.map_err(|e| e.to_string())?);
        }
        Ok(snippets)
    }
}

// --- Головна логіка програми ---

fn main() {

    println!("--- Додаток для нотаток (Snippets App) ---");

    let env_val = env::var("SNIPPETS_APP_STORAGE").unwrap_or_else(|_| {
        println!("Змінна SNIPPETS_APP_STORAGE не встановлена. Приклад: JSON:snippets.json");
        String::new()
    });

    if env_val.is_empty() {
        return;
    }

    let (provider, path) = env_val.split_once(':').expect("Неправильний формат. Використовуйте PROVIDER:PATH");

    let mut storage: Box<dyn SnippetStorage> = match provider {
        "JSON" => Box::new(JsonStorage {
            file_path: path.to_string(),
        }),
        "SQLITE" => Box::new(SqliteStorage::new(path).expect("Не вдалося ініціалізувати базу даних")),
        _ => panic!("Невідомий провайдер: {}", provider),
    };

    let new_snippet = Snippet {
        content: "Привіт із Rust.".to_string(),
        created_at: Utc::now(),
    };
    storage.add(new_snippet).expect("Не вдалося додати нотатку");
    println!("Нотатку успішно додано до сховища {} у файлі {}", provider, path);

    let snippets = storage.list_all().expect("Не вдалося отримати список нотаток");
    println!("Поточні нотатки:");
    for s in snippets {
        println!("- [{}] {}", s.created_at, s.content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_dispatch() {
        let storage = InMemoryStorage::new();
        let mut repo = UserRepositoryStatic::new(storage);

        let user = User {
            id: 1,
            email: Cow::Borrowed("syvolap.eduard@gmail.com"),
            activated: true,
        };

        repo.add(user.clone());
        assert_eq!(repo.get(1), Some(&user));

        let mut updated_user = user.clone();
        updated_user.activated = false;
        repo.update(updated_user.clone());
        assert_eq!(repo.get(1), Some(&updated_user));

        repo.remove(1);
        assert_eq!(repo.get(1), None);
    }

    #[test]
    fn test_dynamic_dispatch() {
        let storage = InMemoryStorage::new();
        let mut repo = UserRepositoryDynamic::new(Box::new(storage));

        let user = User {
            id: 2,
            email: Cow::Borrowed("syvolap.eduard.work@gmail.com"),
            activated: true,
        };

        repo.add(user.clone());
        assert_eq!(repo.get(2), Some(&user));

        repo.remove(2);
        assert_eq!(repo.get(2), None);
    }
}