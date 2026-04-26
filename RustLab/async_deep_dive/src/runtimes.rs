async fn tokio_demo() {
    println!("--- tokio ---");
    println!("  Тип: multi-thread та current-thread executor");
    println!("  Використання: веб-сервери, мікросервіси, CLI інструменти");

    // tokio::spawn — паралельні задачі
    let handles: Vec<_> = (0..4).map(|i| {
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            i * i
        })
    }).collect();

    let mut results = vec![];
    for h in handles {
        results.push(h.await.unwrap());
    }
    println!("  spawn results: {:?}", results);

    // tokio::select! — race між задачами
    let result = tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => "timeout",
        _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => "fast",
    };
    println!("  select! winner: {}", result);

    // tokio::sync канали
    let (tx, mut rx) = tokio::sync::mpsc::channel::<u32>(8);
    tokio::spawn(async move {
        for i in 0..5u32 { tx.send(i).await.unwrap(); }
    });
    let mut sum = 0u32;
    while let Some(v) = rx.recv().await { sum += v; }
    println!("  mpsc channel sum: {}", sum);
}

async fn async_std_demo() {
    println!("\n--- async-std ---");
    println!("  Тип: work-stealing multi-thread executor");
    println!("  Особливість: API дзеркалює std — легко мігрувати");
    println!("  Використання: утиліти, скрипти, невеликі сервіси");

    // async-std spawn
    let handle = async_std::task::spawn(async {
        async_std::task::sleep(std::time::Duration::from_millis(10)).await;
        42u32
    });
    println!("  spawn result: {}", handle.await);

    // async-std channel
    let (tx, rx) = async_std::channel::bounded::<u32>(8);
    async_std::task::spawn(async move {
        for i in 0..3u32 { tx.send(i).await.unwrap(); }
        drop(tx);
    });
    let mut items = vec![];
    while let Ok(v) = rx.recv().await { items.push(v); }
    println!("  channel items: {:?}", items);
}

fn smol_demo() {
    println!("\n--- smol / futures::executor ---");
    println!("  Тип: lightweight single-thread executor");
    println!("  Особливість: мінімальний розмір, без макросів");
    println!("  Використання: embedded, WASM, бібліотеки");

    let result = futures::executor::block_on(async {
        // Простий async розрахунок
        let a = async { 21u32 }.await;
        let b = async { 21u32 }.await;
        a + b
    });
    println!("  block_on result: {}", result);

    // futures::executor::ThreadPool
    let pool = futures::executor::ThreadPool::new().unwrap();
    let (tx, rx) = futures::channel::oneshot::channel::<u32>();

    pool.spawn_ok(async move {
        tx.send(100).unwrap();
    });

    let val = futures::executor::block_on(rx).unwrap();
    println!("  ThreadPool + oneshot: {}", val);
}

fn print_comparison() {
    println!("\n=== Порівняльна таблиця рантаймів ===");
    println!("{:<20} {:<12} {:<12} {:<20}", "Рантайм", "Потоки", "Розмір", "Головне призначення");
    println!("{}", "-".repeat(70));
    println!("{:<20} {:<12} {:<12} {:<20}", "tokio",       "multi",   "~500KB",  "Веб, мікросервіси");
    println!("{:<20} {:<12} {:<12} {:<20}", "async-std",   "multi",   "~400KB",  "Загального призначення");
    println!("{:<20} {:<12} {:<12} {:<20}", "smol",        "single",  "~50KB",   "Embedded, WASM");
    println!("{:<20} {:<12} {:<12} {:<20}", "futures::ex", "single",  "~30KB",   "Бібліотеки, тести");
    println!("{:<20} {:<12} {:<12} {:<20}", "glommio",     "single",  "~200KB",  "io_uring, Linux");
}

#[tokio::main]
async fn main() {
    println!("=== Завдання 1: Порівняння асинхронних рантаймів ===\n");

    tokio_demo().await;
    async_std_demo().await;
    smol_demo();
    print_comparison();
}