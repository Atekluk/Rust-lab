mod my_arc;
mod my_mutex;

use my_arc::MyArc;
use my_mutex::MyMutex;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

const THREADS: usize = 4;
const ITERATIONS: usize = 1_000_000;

fn bench_std_arc() -> u128 {
    let arc = Arc::new(0u64);
    let start = Instant::now();
    let handles: Vec<_> = (0..THREADS).map(|_| {
        let a = Arc::clone(&arc);
        thread::spawn(move || {
            for _ in 0..ITERATIONS {
                let _ = Arc::clone(&a);
            }
        })
    }).collect();
    for h in handles { h.join().unwrap(); }
    start.elapsed().as_millis()
}

fn bench_my_arc() -> u128 {
    let arc = MyArc::new(0u64);
    let start = Instant::now();
    let handles: Vec<_> = (0..THREADS).map(|_| {
        let a = MyArc::clone(&arc);
        thread::spawn(move || {
            for _ in 0..ITERATIONS {
                let _ = MyArc::clone(&a);
            }
        })
    }).collect();
    for h in handles { h.join().unwrap(); }
    start.elapsed().as_millis()
}

fn bench_std_mutex() -> u128 {
    let mutex = Arc::new(Mutex::new(0u64));
    let start = Instant::now();
    let handles: Vec<_> = (0..THREADS).map(|_| {
        let m = Arc::clone(&mutex);
        thread::spawn(move || {
            for _ in 0..ITERATIONS {
                *m.lock().unwrap() += 1;
            }
        })
    }).collect();
    for h in handles { h.join().unwrap(); }
    let elapsed = start.elapsed().as_millis();
    println!("  std::Mutex final value: {}", *mutex.lock().unwrap());
    elapsed
}

fn bench_my_mutex() -> u128 {
    let mutex = MyArc::new(MyMutex::new(0u64));
    let start = Instant::now();
    let handles: Vec<_> = (0..THREADS).map(|_| {
        let m = MyArc::clone(&mutex);
        thread::spawn(move || {
            for _ in 0..ITERATIONS {
                *m.lock() += 1;
            }
        })
    }).collect();
    for h in handles { h.join().unwrap(); }
    let elapsed = start.elapsed().as_millis();
    println!("  MyMutex final value: {}", *mutex.lock());
    elapsed
}

fn main() {
    println!("=== Arc Benchmark ({} threads, {} iterations each) ===\n",
        THREADS, ITERATIONS);

    println!("Running std::Arc...");
    let std_arc = bench_std_arc();
    println!("  std::Arc time: {}ms", std_arc);

    println!("Running MyArc...");
    let my_arc = bench_my_arc();
    println!("  MyArc time:    {}ms", my_arc);

    let arc_diff = if std_arc > my_arc {
        format!("MyArc is {:.1}% faster", (std_arc - my_arc) as f64 / std_arc as f64 * 100.0)
    } else {
        format!("MyArc is {:.1}% slower", (my_arc - std_arc) as f64 / std_arc as f64 * 100.0)
    };
    println!("  Result: {}\n", arc_diff);

    println!("=== Mutex Benchmark ({} threads, {} iterations each) ===\n",
        THREADS, ITERATIONS);

    println!("Running std::Mutex...");
    let std_mutex = bench_std_mutex();
    println!("  std::Mutex time: {}ms", std_mutex);

    println!("Running MyMutex...");
    let my_mutex = bench_my_mutex();
    println!("  MyMutex time:    {}ms", my_mutex);

    let mutex_diff = if std_mutex > my_mutex {
        format!("MyMutex is {:.1}% faster", (std_mutex - my_mutex) as f64 / std_mutex as f64 * 100.0)
    } else {
        format!("MyMutex is {:.1}% slower", (my_mutex - std_mutex) as f64 / std_mutex as f64 * 100.0)
    };
    println!("  Result: {}\n", mutex_diff);
}