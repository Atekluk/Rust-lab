use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

fn demo_cell() {
    println!("--- Cell<T> ---");
    let x = Cell::new(5);
    let r = &x; 
    r.set(10); 
    println!("Cell value after mutation via &ref: {}", x.get());
}

fn demo_refcell() {
    println!("--- RefCell<T> ---");
    let data = RefCell::new(vec![1, 2, 3]);
    {
        let mut v = data.borrow_mut();
        v.push(4);
    } 
    println!("RefCell value: {:?}", data.borrow());

    let result = std::panic::catch_unwind(|| {
        let r = RefCell::new(0);
        let _b1 = r.borrow_mut();
        let _b2 = r.borrow_mut();
    });
    println!("Double borrow_mut panics: {}", result.is_err());
}

fn demo_rc_refcell() {
    println!("--- Rc<RefCell<T>> ---");
    use std::rc::Rc;
    let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
    let clone1 = Rc::clone(&shared);
    let clone2 = Rc::clone(&shared);

    clone1.borrow_mut().push(4);
    clone2.borrow_mut().push(5);

    println!("Shared via Rc<RefCell>: {:?}", shared.borrow());
    println!("Rc strong count: {}", Rc::strong_count(&shared));
}

fn demo_arc() {
    println!("--- Arc<T> ---");
    let data = Arc::new(vec![1, 2, 3]);
    let clone = Arc::clone(&data);

    let handle = thread::spawn(move || {
        println!("Arc in thread: {:?}", clone);
    });
    handle.join().unwrap();
    println!("Arc in main: {:?}", data);
    println!("Arc strong count: {}", Arc::strong_count(&data));
}

fn demo_arc_mutex() {
    println!("--- Arc<Mutex<T>> ---");
    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = vec![];

    for _ in 0..4 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                *c.lock().unwrap() += 1;
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    println!("Counter after 4 threads x 1000: {}", *counter.lock().unwrap());
}

fn demo_rwlock() {
    println!("--- Arc<RwLock<T>> ---");
    let data = Arc::new(RwLock::new(vec![1, 2, 3]));
    let mut handles = vec![];

    for i in 0..3 {
        let d = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            let r = d.read().unwrap();
            println!("Reader {}: {:?}", i, *r);
        }));
    }
    for h in handles { h.join().unwrap(); }

    data.write().unwrap().push(4);
    println!("After write: {:?}", data.read().unwrap());
}

fn main() {
    println!("Обхід системи володінь та запозичень ===\n");

    println!("== Однопотоковий контекст ==");
    demo_cell();
    println!();
    demo_refcell();
    println!();
    demo_rc_refcell();

    println!("\n== Багатопотоковий контекст ==");
    demo_arc();
    println!();
    demo_arc_mutex();
    println!();
    demo_rwlock();
}