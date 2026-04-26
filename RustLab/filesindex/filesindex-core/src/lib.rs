#![warn(clippy::missing_errors_doc, clippy::result_large_err)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub tags: Vec<String>,
}

/// Trait for file index storage backends.
pub trait Storage {
    /// Add a file entry to the storage.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the entry could not be saved.
    fn add(&mut self, entry: FileEntry) -> Result<(), StorageError>;

    /// Get file entries matching all provided tags.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the query failed.
    fn get_by_tags(&self, tags: &[String]) -> Result<Vec<FileEntry>, StorageError>;
}

// ─── JSON Storage ─────────────────────────────────────────────────────────────

pub struct JsonStorage {
    path: PathBuf,
    entries: Vec<FileEntry>,
}

impl JsonStorage {
    /// Create a new JsonStorage.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the file cannot be read or parsed.
    pub fn new(path: PathBuf) -> Result<Self, StorageError> {
        let entries = if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            serde_json::from_str(&data)?
        } else {
            Vec::new()
        };
        Ok(JsonStorage { path, entries })
    }

    fn save(&self) -> Result<(), StorageError> {
        let data = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }
}

impl Storage for JsonStorage {
    fn add(&mut self, entry: FileEntry) -> Result<(), StorageError> {
        if !self.entries.iter().any(|e| e.path == entry.path) {
            self.entries.push(entry);
            self.save()?;
        }
        Ok(())
    }

    fn get_by_tags(&self, tags: &[String]) -> Result<Vec<FileEntry>, StorageError> {
        Ok(self.entries
            .iter()
            .filter(|e| tags.iter().all(|t| e.tags.contains(t)))
            .cloned()
            .collect())
    }
}

// ─── SQLite Storage ───────────────────────────────────────────────────────────

pub struct SqliteStorage {
    conn: rusqlite::Connection,
}

impl SqliteStorage {
    /// Create a new SqliteStorage.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the database cannot be opened or tables created.
    pub fn new(path: PathBuf) -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open(&path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY,
                file_id INTEGER,
                tag TEXT NOT NULL,
                FOREIGN KEY(file_id) REFERENCES files(id)
            );",
        )?;
        Ok(SqliteStorage { conn })
    }
}

impl Storage for SqliteStorage {
    fn add(&mut self, entry: FileEntry) -> Result<(), StorageError> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM files WHERE path = ?1",
            [&entry.path],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if exists {
            return Ok(());
        }

        self.conn.execute(
            "INSERT INTO files (path) VALUES (?1)",
            [&entry.path],
        )?;
        let file_id = self.conn.last_insert_rowid();
        for tag in &entry.tags {
            self.conn.execute(
                "INSERT INTO tags (file_id, tag) VALUES (?1, ?2)",
                rusqlite::params![file_id, tag],
            )?;
        }
        Ok(())
    }

    fn get_by_tags(&self, tags: &[String]) -> Result<Vec<FileEntry>, StorageError> {
        let placeholders: Vec<String> = tags.iter().enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let query = format!(
            "SELECT f.path, GROUP_CONCAT(t.tag) FROM files f
             JOIN tags t ON f.id = t.file_id
             WHERE f.id IN (
                 SELECT file_id FROM tags WHERE tag IN ({})
                 GROUP BY file_id HAVING COUNT(DISTINCT tag) = {}
             )
             GROUP BY f.id",
            placeholders.join(","),
            tags.len()
        );
        let mut stmt = self.conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> = tags.iter()
            .map(|t| t as &dyn rusqlite::ToSql)
            .collect();
        let results = stmt.query_map(params.as_slice(), |row| {
            let path: String = row.get(0)?;
            let tags_str: String = row.get(1)?;
            Ok(FileEntry {
                path,
                tags: tags_str.split(',').map(String::from).collect(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
        Ok(results)
    }
}
