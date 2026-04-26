use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

/// Повідомлення для CounterActor
enum CounterMsg {
    Increment(u32),
    Decrement(u32),
    Get(oneshot::Sender<u32>),
    Reset,
}

/// Актор що керує лічильником
struct CounterActor {
    value: u32,
    receiver: mpsc::Receiver<CounterMsg>,
}

impl CounterActor {
    fn new(receiver: mpsc::Receiver<CounterMsg>) -> Self {
        CounterActor { value: 0, receiver }
    }

    async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                CounterMsg::Increment(n) => self.value += n,
                CounterMsg::Decrement(n) => self.value = self.value.saturating_sub(n),
                CounterMsg::Get(reply) => { let _ = reply.send(self.value); }
                CounterMsg::Reset => self.value = 0,
            }
        }
    }
}

/// Handle для взаємодії з CounterActor
#[derive(Clone)]
struct CounterHandle {
    sender: mpsc::Sender<CounterMsg>,
}

impl CounterHandle {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(CounterActor::new(rx).run());
        CounterHandle { sender: tx }
    }

    async fn increment(&self, n: u32) {
        self.sender.send(CounterMsg::Increment(n)).await.unwrap();
    }

    async fn decrement(&self, n: u32) {
        self.sender.send(CounterMsg::Decrement(n)).await.unwrap();
    }

    async fn get(&self) -> u32 {
        let (tx, rx) = oneshot::channel();
        self.sender.send(CounterMsg::Get(tx)).await.unwrap();
        rx.await.unwrap()
    }

    async fn reset(&self) {
        self.sender.send(CounterMsg::Reset).await.unwrap();
    }
}

async fn demo_basic_actor() {
    println!("--- Базовий актор: CounterActor ---");

    let counter = CounterHandle::new();

    // Кілька потоків надсилають повідомлення одночасно
    let mut handles = vec![];
    for i in 0..5 {
        let c = counter.clone();
        handles.push(tokio::spawn(async move {
            c.increment(i + 1).await;
        }));
    }
    for h in handles { h.await.unwrap(); }

    println!("  Після 5 increment(1..5): {}", counter.get().await);
    counter.decrement(5).await;
    println!("  Після decrement(5): {}", counter.get().await);
    counter.reset().await;
    println!("  Після reset: {}", counter.get().await);
    println!();
}

enum KvMsg {
    Set { key: String, value: String },
    Get { key: String, reply: oneshot::Sender<Option<String>> },
    Delete(String),
    Keys(oneshot::Sender<Vec<String>>),
}

struct KvActor {
    store: HashMap<String, String>,
    receiver: mpsc::Receiver<KvMsg>,
}

impl KvActor {
    fn new(receiver: mpsc::Receiver<KvMsg>) -> Self {
        KvActor { store: HashMap::new(), receiver }
    }

    async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                KvMsg::Set { key, value } => { self.store.insert(key, value); }
                KvMsg::Get { key, reply } => { let _ = reply.send(self.store.get(&key).cloned()); }
                KvMsg::Delete(key) => { self.store.remove(&key); }
                KvMsg::Keys(reply) => { let _ = reply.send(self.store.keys().cloned().collect()); }
            }
        }
    }
}

#[derive(Clone)]
struct KvHandle {
    sender: mpsc::Sender<KvMsg>,
}

impl KvHandle {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(KvActor::new(rx).run());
        KvHandle { sender: tx }
    }

    async fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.sender.send(KvMsg::Set { key: key.into(), value: value.into() }).await.unwrap();
    }

    async fn get(&self, key: impl Into<String>) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(KvMsg::Get { key: key.into(), reply: tx }).await.unwrap();
        rx.await.unwrap()
    }

    async fn delete(&self, key: impl Into<String>) {
        self.sender.send(KvMsg::Delete(key.into())).await.unwrap();
    }

    async fn keys(&self) -> Vec<String> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(KvMsg::Keys(tx)).await.unwrap();
        rx.await.unwrap()
    }
}

async fn demo_kv_actor() {
    println!("--- KV Store Actor ---");

    let kv = KvHandle::new();
    kv.set("name", "Alice").await;
    kv.set("age", "30").await;
    kv.set("city", "Kyiv").await;

    println!("  get(name): {:?}", kv.get("name").await);
    println!("  get(age): {:?}", kv.get("age").await);
    println!("  get(missing): {:?}", kv.get("missing").await);

    kv.delete("age").await;
    let mut keys = kv.keys().await;
    keys.sort();
    println!("  keys after delete: {:?}", keys);
    println!();
}

fn compare_actors_vs_async() {
    println!("--- Порівняння: Модель акторів vs Звичайний async ---\n");

    println!("Спільні риси:");
    println!("  - Обидва використовують async/await у Rust");
    println!("  - Обидва уникають data races через Rust ownership");
    println!("  - Обидва підходять для конкурентного виконання\n");

    println!("Відмінності:");
    println!("  {:<25} {:<30} {:<30}", "Аспект", "Актори", "Звичайний async");
    println!("  {}", "-".repeat(85));
    println!("  {:<25} {:<30} {:<30}", "Стан", "Інкапсульований в акторі", "Shared Arc<Mutex<T>>");
    println!("  {:<25} {:<30} {:<30}", "Комунікація", "Повідомлення (mpsc)", "Пряме await");
    println!("  {:<25} {:<30} {:<30}", "Масштабування", "Легко додати акторів", "Складніша координація");
    println!("  {:<25} {:<30} {:<30}", "Відлагодження", "Простіше (ізоляція)", "Складніше (shared state)");
    println!("  {:<25} {:<30} {:<30}", "Overhead", "Канали + повідомлення", "Менший overhead");
    println!("  {:<25} {:<30} {:<30}", "Використання", "Складні системи", "Прості задачі\n");

    println!("Коли використовувати акторів:");
    println!("  - Складний розподілений стан");
    println!("  - Потрібна ізоляція компонентів");
    println!("  - Система подій та реакцій");
    println!("  - Inspired by: Erlang, Akka, xactor крейт\n");
}

#[tokio::main]
async fn main() {
    println!("=== Завдання 3: Модель акторів ===\n");

    demo_basic_actor().await;
    demo_kv_actor().await;
    compare_actors_vs_async();

    println!("All demonstrations completed!");
}