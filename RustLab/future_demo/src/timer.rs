use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

struct SharedState {
    /// Чи минув вказаний час?
    completed: bool,
    waker: Option<Waker>,
}

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

impl TimerFuture {
    pub fn new(duration_ms: u64) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        let state_clone = Arc::clone(&shared_state);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(duration_ms));
            let mut state = state_clone.lock().unwrap();
            state.completed = true;
            if let Some(waker) = state.waker.take() {
                waker.wake();
            }
        });

        TimerFuture { shared_state }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.shared_state.lock().unwrap();

        if state.completed {
            println!("[TimerFuture] Timer fired! Returning Ready.");
            Poll::Ready(())
        } else {
            // Зберігаємо Waker щоб background thread міг нас розбудити
            state.waker = Some(cx.waker().clone());
            println!("[TimerFuture] Not ready yet, returning Pending...");
            Poll::Pending
        }
    }
}

async fn demo_single_timer() {
    println!("--- Single timer: 200ms ---");
    let start = std::time::Instant::now();
    TimerFuture::new(200).await;
    println!("Elapsed: {:.1}ms\n", start.elapsed().as_secs_f64() * 1000.0);
}

async fn demo_sequential_timers() {
    println!("--- Sequential timers: 100ms + 150ms ---");
    let start = std::time::Instant::now();
    TimerFuture::new(100).await;
    println!("First timer done at {:.1}ms", start.elapsed().as_secs_f64() * 1000.0);
    TimerFuture::new(150).await;
    println!("Second timer done at {:.1}ms\n", start.elapsed().as_secs_f64() * 1000.0);
}

async fn demo_concurrent_timers() {
    println!("--- Concurrent timers: 300ms + 100ms + 200ms ---");
    let start = std::time::Instant::now();

    // Запускаємо всі три паралельно через tokio::join!
    tokio::join!(
        async {
            TimerFuture::new(300).await;
            println!("300ms timer done at {:.1}ms", start.elapsed().as_secs_f64() * 1000.0);
        },
        async {
            TimerFuture::new(100).await;
            println!("100ms timer done at {:.1}ms", start.elapsed().as_secs_f64() * 1000.0);
        },
        async {
            TimerFuture::new(200).await;
            println!("200ms timer done at {:.1}ms", start.elapsed().as_secs_f64() * 1000.0);
        },
    );

    println!(
        "All timers done at {:.1}ms (expected ~300ms)\n",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[tokio::main]
async fn main() {
    println!("=== TimerFuture Demo ===\n");

    demo_single_timer().await;
    demo_sequential_timers().await;
    demo_concurrent_timers().await;

    println!("All demos completed!");
}