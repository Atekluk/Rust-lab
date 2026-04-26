use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::runtime::Builder;
use tokio::sync::Semaphore;

#[derive(Parser, Debug)]
#[command(name = "web-downloader")]
#[command(about = "Асинхронно завантажує веб-сторінки зі списку посилань")]
struct Args {
    #[arg(long)]
    max_threads: Option<usize>,

    #[arg(long, default_value = "8")]
    max_concurrent: usize,

    /// Файл зі списком URL
    file: Option<PathBuf>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct DownloadResult {
    url: String,
    bytes: usize,
    filename: String,
}

async fn download(
    client: reqwest::Client,
    url: String,
    semaphore: Arc<Semaphore>,
) -> Result<DownloadResult, String> {
    let _permit = semaphore.acquire().await.map_err(|e| e.to_string())?;

    println!("[→] Завантаження: {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("{}: {}", url, e))?;

    let status = response.status();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("{}: {}", url, e))?;

    let filename = url
        .replace("https://", "")
        .replace("http://", "")
        .replace('/', "_")
        .replace('?', "_")
        .replace('&', "_")
        .replace(':', "_");
    let filename = format!("output/{}.html", &filename[..filename.len().min(80)]);

    fs::create_dir_all("output")
        .await
        .map_err(|e| format!("create_dir: {}", e))?;

    let byte_count = bytes.len();
    fs::write(&filename, &bytes)
        .await
        .map_err(|e| format!("write {}: {}", filename, e))?;

    println!("[✓] {} — {} ({} байт) → {}", url, status, byte_count, filename);

    Ok(DownloadResult {
        url,
        bytes: byte_count,
        filename,
    })
}

fn read_urls(file: Option<PathBuf>) -> Vec<String> {
    match file {
        Some(path) => {
            let f = std::fs::File::open(&path).expect("Не вдалось відкрити файл");
            io::BufReader::new(f)
                .lines()
                .filter_map(|l| l.ok())
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect()
        }
        None => {
            println!("Введіть URL (по одному на рядок, Ctrl+D для завершення):");
            io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        }
    }
}

fn main() {
    let args = Args::parse();

    let num_threads = args.max_threads.unwrap_or_else(num_cpus);
    let max_concurrent = args.max_concurrent;

    println!("Запуск з {} потоками, max {} одночасних завантажень",
        num_threads, max_concurrent);

    let runtime = Builder::new_multi_thread()
        .worker_threads(num_threads)
        .enable_all()
        .build()
        .expect("Не вдалось створити runtime");

    runtime.block_on(async {
        let urls = read_urls(args.file);

        if urls.is_empty() {
            eprintln!("Список URL порожній!");
            return;
        }

        println!("Завантаження {} URL...\n", urls.len());

        let client = reqwest::Client::builder()
            .user_agent("web-downloader/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Не вдалось створити HTTP клієнт");

        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        let mut futures: FuturesUnordered<_> = urls
            .into_iter()
            .map(|url| {
                let client = client.clone();
                let sem = Arc::clone(&semaphore);
                tokio::spawn(download(client, url, sem))
            })
            .collect();

        let mut success = 0;
        let mut failed = 0;
        let mut total_bytes = 0;

        // Обробляє результати по мірі їх готовності
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(r)) => {
                    success += 1;
                    total_bytes += r.bytes;
                }
                Ok(Err(e)) => {
                    eprintln!("[✗] {}", e);
                    failed += 1;
                }
                Err(e) => {
                    eprintln!("[✗] Task panic: {}", e);
                    failed += 1;
                }
            }
        }

        println!("\n=== Результат ===");
        println!("Успішно:  {}", success);
        println!("Помилок:  {}", failed);
        println!("Всього:   {} байт", total_bytes);
        println!("Готово!");
    });
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}