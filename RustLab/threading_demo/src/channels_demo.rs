use std::sync::mpsc;
use std::thread;
use std::time::Instant;

const MESSAGES: usize = 1_000_000;

fn bench_std_mpsc() -> u128 {
    println!("--- std::sync::mpsc ---");
    println!("  Тип: Multi-Producer Single-Consumer");
    println!("  Особливість: вбудований у std, простий у використанні");

    let (tx, rx) = mpsc::channel::<u32>();
    let start = Instant::now();

    let sender = thread::spawn(move || {
        for i in 0..MESSAGES as u32 {
            tx.send(i).unwrap();
        }
    });

    let receiver = thread::spawn(move || {
        let mut sum = 0u64;
        for _ in 0..MESSAGES {
            sum += rx.recv().unwrap() as u64;
        }
        sum
    });

    sender.join().unwrap();
    let sum = receiver.join().unwrap();
    let elapsed = start.elapsed().as_millis();
    println!("  {} повідомлень, sum={}, час={}ms\n", MESSAGES, sum, elapsed);
    elapsed
}

fn demo_std_mpsc_multi_producer() {
    println!("--- std::sync::mpsc: Multiple Producers ---");
    let (tx, rx) = mpsc::channel::<String>();
    let mut handles = vec![];

    for i in 0..4 {
        let tx_clone = tx.clone();
        handles.push(thread::spawn(move || {
            for j in 0..3 {
                tx_clone.send(format!("Thread {} msg {}", i, j)).unwrap();
            }
        }));
    }

    drop(tx);
    for h in handles { h.join().unwrap(); }

    let mut messages = vec![];
    while let Ok(msg) = rx.recv() {
        messages.push(msg);
    }
    println!("  Отримано {} повідомлень від 4 продюсерів\n", messages.len());
}

fn bench_crossbeam() -> u128 {
    println!("--- crossbeam-channel ---");
    println!("  Тип: Multi-Producer Multi-Consumer");
    println!("  Особливість: швидший за std, підтримує select!, bounded/unbounded");

    let (tx, rx) = crossbeam_channel::unbounded::<u32>();
    let start = Instant::now();

    let sender = thread::spawn(move || {
        for i in 0..MESSAGES as u32 {
            tx.send(i).unwrap();
        }
    });

    let receiver = thread::spawn(move || {
        let mut sum = 0u64;
        for _ in 0..MESSAGES {
            sum += rx.recv().unwrap() as u64;
        }
        sum
    });

    sender.join().unwrap();
    let sum = receiver.join().unwrap();
    let elapsed = start.elapsed().as_millis();
    println!("  {} повідомлень, sum={}, час={}ms\n", MESSAGES, sum, elapsed);
    elapsed
}

fn demo_crossbeam_bounded() {
    println!("--- crossbeam-channel: bounded канал (backpressure) ---");
    let (tx, rx) = crossbeam_channel::bounded::<u32>(10); // буфер 10

    let sender = thread::spawn(move || {
        for i in 0..20u32 {
            tx.send(i).unwrap();
            println!("  Sent: {}", i);
        }
    });

    thread::sleep(std::time::Duration::from_millis(5));

    let receiver = thread::spawn(move || {
        for _ in 0..20 {
            let val = rx.recv().unwrap();
            println!("  Recv: {}", val);
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();
    println!();
}

fn demo_crossbeam_select() {
    println!("--- crossbeam-channel: select! macro ---");
    let (tx1, rx1) = crossbeam_channel::unbounded::<&str>();
    let (tx2, rx2) = crossbeam_channel::unbounded::<u32>();

    tx1.send("hello").unwrap();
    tx2.send(42).unwrap();

    for _ in 0..2 {
        crossbeam_channel::select! {
            recv(rx1) -> msg => println!("  rx1: {:?}", msg),
            recv(rx2) -> msg => println!("  rx2: {:?}", msg),
        }
    }
    println!();
}

fn bench_flume() -> u128 {
    println!("--- flume ---");
    println!("  Тип: Multi-Producer Multi-Consumer, async-ready");
    println!("  Особливість: працює і в sync і в async контексті");

    let (tx, rx) = flume::bounded::<u32>(1024);
    let start = Instant::now();

    let sender = thread::spawn(move || {
        for i in 0..MESSAGES as u32 {
            if tx.send(i).is_err() { break; }
        }
    });

    let receiver = thread::spawn(move || {
        let mut sum = 0u64;
        for val in rx.iter() {
            sum += val as u64;
        }
        sum
    });

    sender.join().unwrap();
    let sum = receiver.join().unwrap();
    let elapsed = start.elapsed().as_millis();
    println!("  {} повідомлень, sum={}, час={}ms\n", MESSAGES, sum, elapsed);
    elapsed
}

fn demo_flume_mpmc() {
    println!("--- flume: Multi-Producer Multi-Consumer ---");
    let (tx, rx) = flume::unbounded::<u32>();
    let rx2 = rx.clone();
    let mut handles = vec![];

    for i in 0..3u32 {
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            for j in 0..5u32 {
                tx.send(i * 10 + j).unwrap();
            }
        }));
    }
    drop(tx);
    for h in handles { h.join().unwrap(); }

    let c1 = thread::spawn(move || {
        let mut count = 0;
        while let Ok(_) = rx.recv() { count += 1; }
        println!("  Consumer 1 отримав: {} повідомлень", count);
    });
    let c2 = thread::spawn(move || {
        let mut count = 0;
        while let Ok(_) = rx2.recv() { count += 1; }
        println!("  Consumer 2 отримав: {} повідомлень", count);
    });

    c1.join().unwrap();
    c2.join().unwrap();
    println!("  Разом: 15 повідомлень між 2 консюмерами\n");
}

fn main() {
    println!("=== Завдання 3: Порівняння каналів у Rust ===\n");

    let std_time = bench_std_mpsc();
    demo_std_mpsc_multi_producer();

    let crossbeam_time = bench_crossbeam();
    demo_crossbeam_bounded();
    demo_crossbeam_select();

    let flume_time = bench_flume();
    demo_flume_mpmc();

    println!("=== Підсумок бенчмарку ({} повідомлень) ===", MESSAGES);
    println!("std::mpsc:          {}ms", std_time);
    println!("crossbeam-channel:  {}ms", crossbeam_time);
    println!("flume:              {}ms", flume_time);

    let best = std_time.min(crossbeam_time).min(flume_time);
    if best == crossbeam_time {
        println!("Найшвидший: crossbeam-channel");
    } else if best == flume_time {
        println!("Найшвидший: flume");
    } else {
        println!("Найшвидший: std::mpsc");
    }
}