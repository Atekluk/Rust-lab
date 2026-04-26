use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── CLI

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

// ─── Data

#[derive(Serialize, Deserialize, Clone)]
struct FileEntry {
    path: String,
    tags: Vec<String>,
}

// ─── Trait (поліморфізм)

trait Storage {
    fn add(&mut self, entry: FileEntry);
    fn get_by_tags(&self, tags: &[String]) -> Vec<FileEntry>;
}

// ─── JSON Storage

struct JsonStorage {
    path: PathBuf,
    entries: Vec<FileEntry>,
}

impl JsonStorage {
    fn new(path: PathBuf) -> Self {
        let entries = if path.exists() {
            let data = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };
        JsonStorage { path, entries }
    }

    fn save(&self) {
        let data = serde_json::to_string_pretty(&self.entries).expect("Failed to serialize");
        std::fs::write(&self.path, data).expect("Failed to write JSON");
    }
}

impl Storage for JsonStorage {
    fn add(&mut self, entry: FileEntry) {
        self.entries.push(entry);
        self.save();
    }

    fn get_by_tags(&self, tags: &[String]) -> Vec<FileEntry> {
        self.entries
            .iter()
            .filter(|e| tags.iter().all(|t| e.tags.contains(t)))
            .cloned()
            .collect()
    }
}

// ─── SQLite Storage

struct SqliteStorage {
    conn: rusqlite::Connection,
}

impl SqliteStorage {
    fn new(path: PathBuf) -> Self {
        let conn = rusqlite::Connection::open(&path).expect("Failed to open SQLite");
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
        ).expect("Failed to create tables");
        SqliteStorage { conn }
    }
}

impl Storage for SqliteStorage {
    fn add(&mut self, entry: FileEntry) {
        self.conn.execute(
            "INSERT INTO files (path) VALUES (?1)",
            [&entry.path],
        ).expect("Failed to insert file");
        let file_id = self.conn.last_insert_rowid();
        for tag in &entry.tags {
            self.conn.execute(
                "INSERT INTO tags (file_id, tag) VALUES (?1, ?2)",
                rusqlite::params![file_id, tag],
            ).expect("Failed to insert tag");
        }
    }

    fn get_by_tags(&self, tags: &[String]) -> Vec<FileEntry> {
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
        let mut stmt = self.conn.prepare(&query).expect("Failed to prepare query");
        let params: Vec<&dyn rusqlite::ToSql> = tags.iter()
            .map(|t| t as &dyn rusqlite::ToSql)
            .collect();
        stmt.query_map(params.as_slice(), |row| {
            let path: String = row.get(0)?;
            let tags_str: String = row.get(1)?;
            Ok(FileEntry {
                path,
                tags: tags_str.split(',').map(String::from).collect(),
            })
        })
        .expect("Failed to query")
        .filter_map(|r| r.ok())
        .collect()
    }
}

//main

fn main() {
    let cli = Cli::parse();

    let env_val = std::env::var("FILES_INDEX_PATH")
        .unwrap_or_else(|_| "json:./files_index.json".to_string());

    let mut parts = env_val.splitn(2, ':');
    let storage_type = parts.next().unwrap_or("json");
    let storage_path = parts.next().unwrap_or_else(|| {
        eprintln!("Warning: FILES_INDEX_PATH has invalid format. Expected 'type:path'. Using default.");
        "./files_index.json"
    });

    let mut storage: Box<dyn Storage> = match storage_type {
        "sqlite" => Box::new(SqliteStorage::new(PathBuf::from(storage_path))),
        _ => Box::new(JsonStorage::new(PathBuf::from(storage_path))),
    };

    match cli.command {
        Command::Add { path, tags } => {
            let tags: Vec<String> = tags.split(',').map(String::from).collect();
            let entry = FileEntry {
                path: path.to_string_lossy().to_string(),
                tags,
            };
            storage.add(entry);
            println!("Added: {}", path.display());
        }
        Command::Get { tags } => {
            let tags: Vec<String> = tags.split(',').map(String::from).collect();
            let results = storage.get_by_tags(&tags);
            if results.is_empty() {
                println!("No files found");
            } else {
                for entry in results {
                    println!("{} [{}]", entry.path, entry.tags.join(", "));
                }
            }
        }
    }
}