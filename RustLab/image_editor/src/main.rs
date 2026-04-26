#![warn(
    missing_docs,
    rustdoc::missing_crate_level_docs,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::result_large_err
)]

//! # image_editor
//!
//! CLI утиліта для зміни розміру зображень із файлу зі списком.
//! IO-bound задачі (завантаження/збереження) виконуються асинхронно через tokio.
//! CPU-bound задачі (resize) виконуються у blocking thread pool.

use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use image::imageops::FilterType;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};
use thiserror::Error;
use tokio::fs;
use tokio::sync::Semaphore;

/// Помилки які можуть виникнути під час роботи програми.
#[derive(Debug, Error)]
pub enum AppError {
    /// Помилка читання або запису файлу.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Помилка завантаження зображення з мережі.
    #[error("Download error: {0}")]
    Download(String),

    /// Помилка декодування або обробки зображення.
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    /// Помилка завантаження на S3.
    #[error("S3 upload error: {status} - {message}")]
    S3Upload {
        /// HTTP статус код відповіді.
        status: u16,
        /// Повідомлення про помилку.
        message: String,
    },

    /// Невірний формат розміру зображення.
    #[error("Invalid resize format: expected WxH, got '{0}'")]
    InvalidSize(String),
}

/// Аргументи командного рядка.
#[derive(Parser)]
#[command(name = "image_editor")]
#[command(about = "Resize images from file list")]
pub struct Cli {
    /// Шлях до файлу зі списком зображень.
    #[arg(long)]
    pub files: PathBuf,

    /// Розмір у форматі WxH (наприклад 100x100).
    #[arg(long)]
    pub resize: String,
}

/// Парсує рядок розміру у форматі `WxH`.
/// # Errors
/// Повертає [`AppError::InvalidSize`] якщо формат невірний.
pub fn parse_size(resize: &str) -> Result<(u32, u32), AppError> {
    let parts: Vec<&str> = resize.split('x').collect();
    if parts.len() != 2 {
        return Err(AppError::InvalidSize(resize.to_string()));
    }
    let width = parts[0].parse::<u32>()
        .map_err(|_| AppError::InvalidSize(resize.to_string()))?;
    let height = parts[1].parse::<u32>()
        .map_err(|_| AppError::InvalidSize(resize.to_string()))?;
    if width == 0 || height == 0 {
        return Err(AppError::InvalidSize(format!(
            "{} (width and height must be greater than 0)", resize
        )));
    }
    Ok((width, height))
}

/// Трейт для завантаження оброблених зображень.
#[async_trait::async_trait]
pub trait Uploader: Send + Sync {
    /// Завантажує файл з вказаною назвою та даними.
    /// # Errors
    /// Повертає [`AppError`] якщо завантаження не вдалось.
    async fn upload(&self, filename: &str, data: Vec<u8>) -> Result<(), AppError>;
}

/// Зберігає зображення у локальну файлову систему.
pub struct FsUploader {
    output_dir: PathBuf,
}

impl FsUploader {
    /// Створює новий `FsUploader`.
    /// # Errors
    /// Повертає [`AppError::Io`] якщо директорію не вдалось створити.
    pub async fn new(path: &str) -> Result<Self, AppError> {
        let output_dir = PathBuf::from(path);
        fs::create_dir_all(&output_dir).await.map_err(|e| {
            AppError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create output directory '{}': {}", path, e),
            ))
        })?;
        Ok(FsUploader { output_dir })
    }
}

#[async_trait::async_trait]
impl Uploader for FsUploader {
    async fn upload(&self, filename: &str, data: Vec<u8>) -> Result<(), AppError> {
        let output_path = self.output_dir.join(filename);
        fs::write(&output_path, data).await?;
        println!("Saved to fs: {}", output_path.display());
        Ok(())
    }
}

/// Завантажує зображення у S3-сумісний bucket через AWS Signature V4.
pub struct S3Uploader {
    bucket: String,
    access_key: String,
    secret_key: String,
    client: reqwest::Client,
}

impl S3Uploader {
    /// Створює новий `S3Uploader`.
    pub fn new(bucket: String) -> Self {
        let access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .expect("AWS_ACCESS_KEY_ID not set");
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .expect("AWS_SECRET_ACCESS_KEY not set");
        let client = reqwest::Client::new();
        S3Uploader { bucket, access_key, secret_key, client }
    }

    fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(key).unwrap();
        mac.update(msg);
        mac.finalize().into_bytes().to_vec()
    }

    fn sha256_hex(data: &[u8]) -> String {
        hex::encode(Sha256::digest(data))
    }
}

#[async_trait::async_trait]
impl Uploader for S3Uploader {
    async fn upload(&self, filename: &str, data: Vec<u8>) -> Result<(), AppError> {
        let now = chrono::Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let region = "auto";
        let service = "s3";
        let host = "t3.storage.dev";

        let content_type = if filename.ends_with(".png") { "image/png" } else { "image/jpeg" };
        let payload_hash = Self::sha256_hex(&data);
        let uri = format!("/{}/{}", self.bucket, filename);

        let canonical_headers = format!(
            "content-type:{}\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            content_type, host, payload_hash, amz_date
        );
        let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "PUT\n{}\n\n{}\n{}\n{}",
            uri, canonical_headers, signed_headers, payload_hash
        );
        let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, region, service);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope,
            Self::sha256_hex(canonical_request.as_bytes())
        );
        let signing_key = {
            let k1 = Self::hmac_sha256(format!("AWS4{}", self.secret_key).as_bytes(), date_stamp.as_bytes());
            let k2 = Self::hmac_sha256(&k1, region.as_bytes());
            let k3 = Self::hmac_sha256(&k2, service.as_bytes());
            Self::hmac_sha256(&k3, b"aws4_request")
        };
        let signature = hex::encode(Self::hmac_sha256(&signing_key, string_to_sign.as_bytes()));
        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={}/{},SignedHeaders={},Signature={}",
            self.access_key, credential_scope, signed_headers, signature
        );

        let url = format!("https://{}{}", host, uri);
        let resp = self.client.put(&url)
            .header("Authorization", &auth)
            .header("Content-Type", content_type)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .body(data)
            .send()
            .await
            .map_err(|e| AppError::Download(e.to_string()))?;

        if resp.status().is_success() {
            println!("Uploaded to S3: {}/{}", self.bucket, filename);
            Ok(())
        } else {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            Err(AppError::S3Upload { status, message })
        }
    }
}

/// Асинхронно завантажує байти зображення з URL або локального файлу.
/// # Errors
/// Повертає [`AppError`] якщо завантаження не вдалось.
pub async fn fetch_image_bytes(source: &str, client: &reqwest::Client) -> Result<Vec<u8>, AppError> {
    if source.starts_with("http://") || source.starts_with("https://") {
        println!("Downloading: {}", source);
        let resp = client.get(source).send().await
            .map_err(|e| AppError::Download(e.to_string()))?;
        let bytes = resp.bytes().await
            .map_err(|e| AppError::Download(e.to_string()))?;
        Ok(bytes.to_vec())
    } else {
        println!("Reading file: {}", source);
        let bytes = fs::read(source).await?;
        Ok(bytes)
    }
}

/// Обробляє одне зображення: асинхронно завантажує,
/// CPU-bound resize у blocking pool, асинхронно зберігає.
/// # Errors
/// Повертає [`AppError`] якщо будь-який крок не вдався.
pub async fn process_image(
    source: String,
    width: u32,
    height: u32,
    uploader: Arc<dyn Uploader>,
    client: reqwest::Client,
) -> Result<(), AppError> {
    let bytes = fetch_image_bytes(&source, &client).await?;

    let encoded = tokio::task::spawn_blocking(move || -> Result<(String, Vec<u8>), AppError> {
        let img = image::load_from_memory(&bytes)?;
        let resized = img.resize_exact(width, height, FilterType::Lanczos3);

        let filename = if source.starts_with("http") {
            source.split('/').next_back().unwrap_or("output.jpg").to_string()
        } else {
            Path::new(&source)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("output.jpg")
                .to_string()
        };

        let ext = Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");
        let format = match ext {
            "png" => image::ImageFormat::Png,
            "jpg" | "jpeg" => image::ImageFormat::Jpeg,
            other => return Err(AppError::InvalidSize(
                format!("Unsupported format: {}", other)
            )),
        };

        let mut buf = Vec::new();
        resized.write_to(&mut std::io::Cursor::new(&mut buf), format)?;
        Ok((filename, buf))
    })
    .await
    .map_err(|e| AppError::Download(e.to_string()))??;

    let (filename, buf) = encoded;
    uploader.upload(&filename, buf).await
}

fn main() {
    let cli = Cli::parse();

    let (width, height) = match parse_size(&cli.resize) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let uploader_type = std::env::var("MYME_UPLOADER").unwrap_or_else(|_| "fs".to_string());
    let num_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

    // Конфігурації для бенчмарку
    let configs: Vec<(&str, usize, usize)> = vec![
        ("1 worker thread",          1,        512),
        ("2 worker threads",         2,        512),
        ("4 worker threads",         4,        512),
        ("cpu_count threads",        num_cpus, 512),
        ("cpu_count*2 threads",      num_cpus * 2, 512),
        ("cpu_count, 64 blocking",   num_cpus, 64),
        ("cpu_count, 256 blocking",  num_cpus, 256),
    ];

    println!("CPU cores: {}", num_cpus);
    println!("Benchmarking different runtime configurations...\n");

    for (label, workers, blocking) in &configs {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(*workers)
            .max_blocking_threads(*blocking)
            .enable_all()
            .build()
            .expect("Failed to build runtime");

        let start = std::time::Instant::now();

        runtime.block_on(async {
            let uploader: std::sync::Arc<dyn Uploader> = match uploader_type.as_str() {
                "s3" => {
                    let bucket = std::env::var("S3_BUCKET")
                        .unwrap_or_else(|_| "image-editor-bucket".to_string());
                    std::sync::Arc::new(S3Uploader::new(bucket)) as std::sync::Arc<dyn Uploader>
                }
                _ => {
                    let path = std::env::var("MYME_FILES_PATH")
                        .unwrap_or_else(|_| "./output".to_string());
                    match FsUploader::new(&path).await {
                        Ok(u) => std::sync::Arc::new(u) as std::sync::Arc<dyn Uploader>,
                        Err(e) => {
                            eprintln!("Error: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            };

            let file = match std::fs::File::open(&cli.files) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error opening file list: {e}");
                    std::process::exit(1);
                }
            };

            let lines: Vec<String> = std::io::BufReader::new(file)
                .lines()
                .filter_map(|l| l.ok())
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            let client = reqwest::Client::builder()
                .user_agent("image-editor/1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client");

            let tasks: Vec<_> = lines.into_iter().map(|line| {
                tokio::spawn(process_image(
                    line, width, height,
                    std::sync::Arc::clone(&uploader),
                    client.clone(),
                ))
            }).collect();

            for task in tasks {
                match task.await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("Error: {e}"),
                    Err(e) => eprintln!("Task error: {e}"),
                }
            }
        });

    let file = match std::fs::File::open(&cli.files) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening file list: {e}");
            std::process::exit(1);
        }
    };

    let lines: Vec<String> = io::BufReader::new(file)
        .lines()
        .filter_map(|l| l.ok())
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    println!("Processing {} images...", lines.len());

    let client = reqwest::Client::builder()
        .user_agent("image-editor/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let semaphore = Arc::new(Semaphore::new(4));
    let mut futures: FuturesUnordered<_> = lines.into_iter().map(|line| {
        let sem = Arc::clone(&semaphore);
        let uploader = Arc::clone(&uploader);
        let client = client.clone();
        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            process_image(line, width, height, uploader, client).await
        })
    }).collect();

    let mut success = 0usize;
    let mut failed = 0usize;

    while let Some(result) = futures.next().await {
        match result {
            Ok(Ok(())) => success += 1,
            Ok(Err(e)) => { eprintln!("Error: {e}"); failed += 1; }
            Err(e) => { eprintln!("Task error: {e}"); failed += 1; }
        }
    }

    println!("Done! Success: {}, Failed: {}", success, failed);
}