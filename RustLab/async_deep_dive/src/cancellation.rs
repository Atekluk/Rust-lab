use std::time::Duration;
use tokio::time::sleep;

async fn demo_cancellation_token() {
    println!("--- Спосіб 1: CancellationToken ---");

    let token = tokio_util::sync::CancellationToken::new();
    let token_clone = token.clone();

    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token_clone.cancelled() => {
                    println!("  Task: отримав сигнал скасування, завершуємо");
                    return "cancelled";
                }
                _ = sleep(Duration::from_millis(50)) => {
                    println!("  Task: виконую роботу...");
                }
            }
        }
    });

    sleep(Duration::from_millis(120)).await;
    println!("  Main: надсилаємо сигнал скасування");
    token.cancel();

    let result = task.await.unwrap();
    println!("  Result: {}\n", result);
}

async fn demo_oneshot_cancel() {
    println!("--- Спосіб 2: oneshot канал як сигнал скасування ---");

    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();

    let task = tokio::spawn(async move {
        let mut cancel_rx = cancel_rx;
        let mut count = 0u32;
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    println!("  Task: oneshot отримано, виконано {} ітерацій", count);
                    break;
                }
                _ = sleep(Duration::from_millis(30)) => {
                    count += 1;
                    println!("  Task: ітерація {}", count);
                }
            }
        }
        count
    });

    sleep(Duration::from_millis(100)).await;
    let _ = cancel_tx.send(());
    let count = task.await.unwrap();
    println!("  Завершено після {} ітерацій\n", count);
}

async fn demo_abort() {
    println!("--- Спосіб 3: JoinHandle::abort() ---");
    println!("  УВАГА: abort() не дає задачі виконати cleanup!");

    let task = tokio::spawn(async {
        println!("  Task: починаю довгу роботу...");
        sleep(Duration::from_secs(10)).await; // довга операція
        println!("  Task: це ніколи не виведеться");
        42u32
    });

    sleep(Duration::from_millis(50)).await;
    println!("  Main: викликаємо abort()");
    task.abort();

    // JoinError::is_cancelled() == true
    println!("  Task aborted successfully\n");
}

async fn demo_timeout() {
    println!("--- Спосіб 4: tokio::time::timeout ---");

    // Успішний випадок
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        async {
            sleep(Duration::from_millis(50)).await;
            "completed"
        }
    ).await;
    println!("  Fast task: {:?}", result);

    // Таймаут
    let result = tokio::time::timeout(
        Duration::from_millis(50),
        async {
            sleep(Duration::from_millis(200)).await;
            "completed"
        }
    ).await;
    println!("  Slow task: {}", if result.is_err() { "timed out!" } else { "ok" });
    println!();
}

fn explain_cancellation_challenges() {
    println!("--- Чому скасування складне у Rust ---\n");

    println!("1. Drop-based cancellation:");
    println!("   Future скасовується коли її drop-ають (не await).");
    println!("   Проміжний стан між await точками може бути некоректним.\n");

    println!("2. Немає async Drop:");
    println!("   Drop є синхронним — не можна виконати async cleanup.");
    println!("   Рішення: явний метод close() або структури Guard.\n");

    println!("3. Cancel safety:");
    println!("   Не всі операції є cancel-safe.");
    println!("   Наприклад: якщо скасувати read() посередині — дані можуть бути втрачені.");
    println!("   tokio документує які методи є cancel-safe.\n");

    println!("4. Structured concurrency:");
    println!("   tokio::task::JoinSet дозволяє керувати групою задач.");
    println!("   abort_all() скасовує всі задачі в групі.\n");
}

#[tokio::main]
async fn main() {
    println!("=== Завдання 2: Скасування асинхронних задач ===\n");

    demo_cancellation_token().await;
    demo_oneshot_cancel().await;
    demo_abort().await;
    demo_timeout().await;
    explain_cancellation_challenges();

    println!("All demonstrations completed!");
}