use std::cell::{Cell, RefCell, UnsafeCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

fn demo_rc_not_send() {
    let rc = Rc::new(42);
    println!("Rc value: {}", rc);

    let arc = Arc::new(42);
    let arc_clone = Arc::clone(&arc);
    let handle = thread::spawn(move || {
        println!("Arc in thread: {}", arc_clone);
    });
    handle.join().unwrap();
    println!("Arc in main: {}", arc);
}

fn demo_mutex_guard_not_send() {
    let mutex = Arc::new(Mutex::new(42));
    let guard = mutex.lock().unwrap();
    println!("MutexGuard value: {}", *guard);

    println!("Mutex unlocked successfully");
}

fn demo_raw_pointer_not_send() {
    let value = 42i32;
    let raw_ptr = &value as *const i32;
    println!("Raw pointer value: {}", unsafe { *raw_ptr });

    let safe_value = Arc::new(42i32);
    let clone = Arc::clone(&safe_value);
    thread::spawn(move || {
        println!("Safe value in thread: {}", clone);
    }).join().unwrap();
}

fn demo_cell_not_sync() {
    let cell = Cell::new(42);
    cell.set(100);
    println!("Cell value: {}", cell.get());

    let mutex_val = Arc::new(Mutex::new(42));
    let clone = Arc::clone(&mutex_val);
    thread::spawn(move || {
        *clone.lock().unwrap() = 999;
        println!("Mutex value set in thread");
    }).join().unwrap();
    println!("Mutex value in main: {}", *mutex_val.lock().unwrap());
}

fn demo_refcell_not_sync() {
    let refcell = RefCell::new(vec![1, 2, 3]);
    refcell.borrow_mut().push(4);
    println!("RefCell value: {:?}", refcell.borrow());

    let shared = Arc::new(Mutex::new(vec![1, 2, 3]));
    let clone = Arc::clone(&shared);
    thread::spawn(move || {
        clone.lock().unwrap().push(4);
        println!("Pushed to vec in thread");
    }).join().unwrap();
    println!("Vec in main: {:?}", shared.lock().unwrap());
}

fn demo_unsafe_cell_not_sync() {
    let unsafe_cell = UnsafeCell::new(42);
    unsafe {
        *unsafe_cell.get() = 100;
        println!("UnsafeCell value: {}", *unsafe_cell.get());
    }


    println!("UnsafeCell can only be used in single-threaded context");
}

fn main() {
    println!("=== !Send types demonstration ===\n");

    println!("--- Rc<T> is !Send ---");
    demo_rc_not_send();

    println!("\n--- MutexGuard is !Send ---");
    demo_mutex_guard_not_send();

    println!("\n--- *mut T is !Send ---");
    demo_raw_pointer_not_send();

    println!("\n=== !Sync types demonstration ===\n");

    println!("--- Cell<T> is !Sync ---");
    demo_cell_not_sync();

    println!("\n--- RefCell<T> is !Sync ---");
    demo_refcell_not_sync();

    println!("\n--- UnsafeCell<T> is !Sync ---");
    demo_unsafe_cell_not_sync();

    println!("\nAll demonstrations completed!");
}