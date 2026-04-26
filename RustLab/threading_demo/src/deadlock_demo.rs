use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn explain_deadlock() {
    println!("--- Deadlock: причина ---");
    println!("Thread 1: захоплює Lock A, чекає Lock B");
    println!("Thread 2: захоплює Lock B, чекає Lock A");
    println!("Результат: обидва чекають вічно\n");
}
fn fix_deadlock_ordering() {
    println!("--- Deadlock Fix #1: впорядкування блокувань ---");

    let lock_a = Arc::new(Mutex::new(0u32));
    let lock_b = Arc::new(Mutex::new(0u32));

    let mut handles = vec![];

    for i in 0..4 {
        let a = Arc::clone(&lock_a);
        let b = Arc::clone(&lock_b);

        handles.push(thread::spawn(move || {
            let mut a_guard = a.lock().unwrap();
            let mut b_guard = b.lock().unwrap();
            *a_guard += i;
            *b_guard += i;
            println!("Thread {}: a={}, b={}", i, *a_guard, *b_guard);
        }));
    }

    for h in handles { h.join().unwrap(); }
    println!("No deadlock! Final: a={}, b={}\n",
        *lock_a.lock().unwrap(), *lock_b.lock().unwrap());
}

fn fix_deadlock_trylock() {
    println!("--- Deadlock Fix #2: try_lock ---");

    let lock_a = Arc::new(Mutex::new(0u32));
    let lock_b = Arc::new(Mutex::new(0u32));
    let mut handles = vec![];

    for i in 0..3 {
        let a = Arc::clone(&lock_a);
        let b = Arc::clone(&lock_b);

        handles.push(thread::spawn(move || {
            loop {
                if let Ok(mut a_guard) = a.try_lock() {
                    if let Ok(mut b_guard) = b.try_lock() {
                        *a_guard += 1;
                        *b_guard += 1;
                        println!("Thread {}: успішно захопив обидва", i);
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        }));
    }

    for h in handles { h.join().unwrap(); }
    println!("Done! a={}, b={}\n",
        *lock_a.lock().unwrap(), *lock_b.lock().unwrap());
}

fn demo_starvation_prevention() {
    println!("--- Resource Starvation: справедливий доступ через Mutex + черга ---");

    let queue = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let counter = Arc::new(Mutex::new(0u32));
    let mut handles = vec![];

    // Заповнюємо чергу заздалегідь
    {
        let mut q = queue.lock().unwrap();
        for i in 0u32..5 { q.push_back(i); }
    }

    for i in 0..5u32 {
        let queue = Arc::clone(&queue);
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            loop {
                if let Ok(mut q) = queue.try_lock() {
                    if let Some(id) = q.pop_front() {
                        if id == i {
                            let mut c = counter.lock().unwrap();
                            *c += 1;
                            println!("Thread {} отримав доступ, counter={}", i, *c);
                            break;
                        } else {
                            q.push_back(id); // повертаємо назад
                        }
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        }));
    }

    for h in handles { h.join().unwrap(); }
    println!("Final counter: {}\n", *counter.lock().unwrap());
}

fn main() {
    println!("=== Завдання 1: Проблеми багатопотокового виконання ===\n");

    explain_deadlock();
    fix_deadlock_ordering();
    fix_deadlock_trylock();
    demo_starvation_prevention();

    println!("All demonstrations completed!");
}