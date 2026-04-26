use std::sync::mpsc;
use std::thread;
use rayon::prelude::*;

const SIZE: usize = 4096;

fn generate_matrix() -> Vec<Vec<i64>> {
    (0..SIZE)
        .map(|i| (0..SIZE).map(|j| (i * SIZE + j) as i64).collect())
        .collect()
}

fn main() {
    let (tx1, rx1) = mpsc::channel::<Vec<Vec<i64>>>();
    let (tx2, rx2) = mpsc::channel::<Vec<Vec<i64>>>();

    // Потік-генератор
    let generator = thread::spawn(move || {
        println!("Generator: creating matrix {}x{}...", SIZE, SIZE);
        let matrix = generate_matrix();
        tx1.send(matrix.clone()).expect("Failed to send to thread 1");
        tx2.send(matrix).expect("Failed to send to thread 2");
        println!("Generator: matrix sent to both threads");
    });

    // Потік 1 — рахує суму парних рядків
    let counter1 = thread::spawn(move || {
        let matrix = rx1.recv().expect("Failed to receive matrix");
        let sum: i64 = matrix.par_iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, row)| row.par_iter().sum::<i64>())
            .sum();
        println!("Thread 1 (even rows sum): {}", sum);
        sum
    });

    // Потік 2 — рахує суму непарних рядків
    let counter2 = thread::spawn(move || {
        let matrix = rx2.recv().expect("Failed to receive matrix");
        let sum: i64 = matrix.par_iter()
            .enumerate()
            .filter(|(i, _)| i % 2 != 0)
            .map(|(_, row)| row.par_iter().sum::<i64>())
            .sum();
        println!("Thread 2 (odd rows sum): {}", sum);
        sum
    });

    generator.join().expect("Generator panicked");
    let sum1 = counter1.join().expect("Counter 1 panicked");
    let sum2 = counter2.join().expect("Counter 2 panicked");

    println!("Total sum: {}", sum1 + sum2);
}
