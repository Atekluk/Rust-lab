use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

struct MeasurableFuture<Fut> {
    inner_future: Fut,
    started_at: Option<Instant>,
}

impl<Fut> MeasurableFuture<Fut> {
    fn new(fut: Fut) -> Self {
        MeasurableFuture {
            inner_future: fut,
            started_at: None,
        }
    }
}

impl<Fut> Future for MeasurableFuture<Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {

        let this = unsafe { self.get_unchecked_mut() };

        if this.started_at.is_none() {
            this.started_at = Some(Instant::now());
            println!("[MeasurableFuture] Started measuring...");
        }

        let inner_pin = unsafe { Pin::new_unchecked(&mut this.inner_future) };

        match inner_pin.poll(cx) {
            Poll::Ready(output) => {
                let elapsed = this.started_at.unwrap().elapsed();
                println!(
                    "[MeasurableFuture] Completed in {:.3}ms ({:.6}s)",
                    elapsed.as_secs_f64() * 1000.0,
                    elapsed.as_secs_f64()
                );
                Poll::Ready(output)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn fast_task() -> u32 {
    println!("  [fast_task] Running...");
    42
}

async fn slow_task() -> String {
    println!("  [slow_task] Starting...");
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    println!("  [slow_task] Done!");
    String::from("hello from slow task")
}

async fn network_simulation() -> Vec<u32> {
    println!("  [network_simulation] Fetching data...");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("  [network_simulation] Data received!");
    vec![1, 2, 3, 4, 5]
}

#[tokio::main]
async fn main() {
    println!("=== MeasurableFuture Demo ===\n");

    println!("--- Test 1: fast task ---");
    let result = MeasurableFuture::new(fast_task()).await;
    println!("Result: {}\n", result);

    println!("--- Test 2: slow task (150ms) ---");
    let result = MeasurableFuture::new(slow_task()).await;
    println!("Result: {}\n", result);

    println!("--- Test 3: network simulation (300ms) ---");
    let result = MeasurableFuture::new(network_simulation()).await;
    println!("Result: {:?}\n", result);

    println!("--- Test 4: nested MeasurableFuture ---");
    let result = MeasurableFuture::new(
        MeasurableFuture::new(slow_task())
    ).await;
    println!("Result: {}\n", result);
}