use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn demo_atomic_counter() {
    println!("--- Atomic counter vs Mutex counter ---");

    let num_threads = 8;
    let iterations = 100_000;

    // Atomic — lock-free
    let atomic_counter = Arc::new(AtomicUsize::new(0));
    let start = std::time::Instant::now();
    let mut handles = vec![];

    for _ in 0..num_threads {
        let c = Arc::clone(&atomic_counter);
        handles.push(thread::spawn(move || {
            for _ in 0..iterations {
                c.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    let atomic_time = start.elapsed();
    println!("Atomic:  {} in {:.1}ms", atomic_counter.load(Ordering::SeqCst), atomic_time.as_secs_f64() * 1000.0);

    // Mutex — blocking
    let mutex_counter = Arc::new(std::sync::Mutex::new(0usize));
    let start = std::time::Instant::now();
    let mut handles = vec![];

    for _ in 0..num_threads {
        let c = Arc::clone(&mutex_counter);
        handles.push(thread::spawn(move || {
            for _ in 0..iterations {
                *c.lock().unwrap() += 1;
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    let mutex_time = start.elapsed();
    println!("Mutex:   {} in {:.1}ms", *mutex_counter.lock().unwrap(), mutex_time.as_secs_f64() * 1000.0);

    let speedup = mutex_time.as_secs_f64() / atomic_time.as_secs_f64();
    println!("Atomic є {:.1}x швидшим за Mutex\n", speedup);
}

fn demo_atomic_flag() {
    println!("--- AtomicBool: lock-free прапорець зупинки ---");

    let running = Arc::new(AtomicBool::new(true));
    let counter = Arc::new(AtomicUsize::new(0));

    let running_clone = Arc::clone(&running);
    let counter_clone = Arc::clone(&counter);

    let worker = thread::spawn(move || {
        while running_clone.load(Ordering::Acquire) {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            thread::yield_now();
        }
        println!("Worker зупинився після {} ітерацій",
            counter_clone.load(Ordering::SeqCst));
    });

    thread::sleep(std::time::Duration::from_millis(10));
    running.store(false, Ordering::Release);
    worker.join().unwrap();
    println!("Головний потік: worker зупинено\n");
}

fn demo_cas() {
    println!("--- Compare-And-Swap (CAS) операція ---");

    let value = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..4 {
        let v = Arc::clone(&value);
        let s = Arc::clone(&success_count);

        handles.push(thread::spawn(move || {
            match v.compare_exchange(0, i + 1, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(old) => {
                    s.fetch_add(1, Ordering::Relaxed);
                    println!("Thread {}: CAS успіх! {} → {}", i, old, i + 1);
                }
                Err(current) => {
                    println!("Thread {}: CAS провал, поточне значення: {}", i, current);
                }
            }
        }));
    }

    for h in handles { h.join().unwrap(); }
    println!("Успішних CAS: {}/4", success_count.load(Ordering::SeqCst));
    println!("Фінальне значення: {}\n", value.load(Ordering::SeqCst));
}

fn demo_memory_ordering() {
    println!("--- Memory Ordering: Relaxed vs SeqCst продуктивність ---");

    let counter = Arc::new(AtomicUsize::new(0));
    let iterations = 1_000_000;

    // Relaxed — найшвидший, без гарантій порядку
    let c = Arc::clone(&counter);
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        c.fetch_add(1, Ordering::Relaxed);
    }
    let relaxed_time = start.elapsed();
    println!("Relaxed:  {:.2}ms", relaxed_time.as_secs_f64() * 1000.0);

    counter.store(0, Ordering::SeqCst);

    // SeqCst — найповільніший, повна послідовність
    let c = Arc::clone(&counter);
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        c.fetch_add(1, Ordering::SeqCst);
    }
    let seqcst_time = start.elapsed();
    println!("SeqCst:   {:.2}ms", seqcst_time.as_secs_f64() * 1000.0);

    println!("SeqCst на {:.1}x повільніший за Relaxed\n",
        seqcst_time.as_secs_f64() / relaxed_time.as_secs_f64());
}

fn main() {
    println!("=== Завдання 2: Неблокуючі структури даних ===\n");

    demo_atomic_counter();
    demo_atomic_flag();
    demo_cas();
    demo_memory_ordering();

    println!("All demonstrations completed!");
}