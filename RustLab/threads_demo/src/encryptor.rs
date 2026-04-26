use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use clap::Parser;
use rand::Rng;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "encryptor")]
struct Cli {
    /// Directory to encrypt
    #[arg(long)]
    dir: String,
}

struct FileData {
    name: String,
    content: Vec<u8>,
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn encrypt(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut encrypted = cipher.encrypt(nonce, data).expect("Encryption failed");
    let mut result = nonce_bytes.to_vec();
    result.append(&mut encrypted);
    result
}

fn main() {
    let cli = Cli::parse();
    let key: [u8; 32] = rand::thread_rng().gen();
    let key = Arc::new(key);

    let (tx, rx) = mpsc::channel::<FileData>();
    let rx = Arc::new(std::sync::Mutex::new(rx));

    // Потік-читач — рекурсивно читає файли
    let dir = cli.dir.clone();
    let reader = thread::spawn(move || {
        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path().to_owned();
                let name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                match fs::read(&path) {
                    Ok(content) => {
                        println!("Reader: sending {}", name);
                        tx.send(FileData { name, content }).ok();
                    }
                    Err(e) => eprintln!("Failed to read {}: {}", name, e),
                }
            }
        }
        println!("Reader: done");
    });

    // 3 потоки-шифрувальники
    let mut workers = vec![];
    for id in 0..3 {
        let rx = Arc::clone(&rx);
        let key = Arc::clone(&key);
        let worker = thread::spawn(move || {
            loop {
                let file = {
                    let rx = rx.lock().unwrap();
                    rx.recv()
                };
                match file {
                    Ok(f) => {
                        let encrypted = encrypt(&f.content, &key);
                        let out_name = format!("{}.data", f.name);
                        fs::write(&out_name, encrypted)
                            .expect("Failed to write encrypted file");
                        COUNTER.fetch_add(1, Ordering::SeqCst);
                        println!("Worker {}: encrypted {} -> {}", id, f.name, out_name);
                    }
                    Err(_) => break,
                }
            }
        });
        workers.push(worker);
    }

    let monitor = thread::spawn(|| {
        let mut last = 0;
        loop {
            let current = COUNTER.load(Ordering::SeqCst);
            if current != last {
                println!("Monitor: processed {} files", current);
                last = current;
            }
            thread::sleep(std::time::Duration::from_millis(10));
            if current == last && current > 0 {
                static NO_CHANGE: AtomicUsize = AtomicUsize::new(0);
                NO_CHANGE.fetch_add(1, Ordering::SeqCst);
                if NO_CHANGE.load(Ordering::SeqCst) > 100 {
                    break;
                }
            }
        }
    });

    reader.join().expect("Reader panicked");
    for w in workers {
        w.join().expect("Worker panicked");
    }
    monitor.join().expect("Monitor panicked");

    println!("Done! Total files encrypted: {}", COUNTER.load(Ordering::SeqCst));
}
