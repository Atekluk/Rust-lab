macro_rules! hashmap {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = std::collections::HashMap::new();
        $(map.insert($key, $val);)*
        map
    }};
}

macro_rules! log_info {
    ($($arg:tt)*) => {
        println!("[INFO] {}", format!($($arg)*));
    };
}

macro_rules! make_getter {
    ($field:ident: $type:ty) => {
        pub fn $field(&self) -> &$type {
            &self.$field
        }
    };
}

struct Person {
    name: String,
    age: u32,
}

impl Person {
    make_getter!(name: String);
    make_getter!(age: u32);

    fn new(name: &str, age: u32) -> Self {
        Person { name: name.to_string(), age }
    }
}

fn demo_declarative_macros() {
    println!("Декларативні макроси");

    let v = vec![1, 2, 3, 4, 5];
    println!("vec!: {:?}", v);

    assert!(v.len() == 5, "Length must be 5");
    assert_eq!(v[0], 1);
    println!("assert! та assert_eq! пройшли");

    let map = hashmap! {
        "one" => 1,
        "two" => 2,
        "three" => 3,
    };
    println!("hashmap!: {:?}", map.get("two"));

    log_info!("Processing {} items", v.len());

    let person = Person::new("Alice", 30);
    println!("Person name: {}, age: {}", person.name(), person.age());
}

#[derive(Debug, Clone, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Default)]
struct Config {
    width: u32,
    height: u32,
    title: String,
}

fn demo_derive_macros() {
    println!("Процедурні derive макроси");

    let p = Point { x: 1.0, y: 2.0 };
    println!("Debug: {:?}", p);

    let p2 = p.clone();
    println!("Clone: {:?}", p2);

    println!("p == p2: {}", p == p2);
    let p3 = Point { x: 3.0, y: 4.0 };
    println!("p == p3: {}", p == p3);

    let config = Config::default();
    println!("Default Config: {:?}", config);

    let custom = Config {
        width: 1920,
        height: 1080,
        title: String::from("My App"),
        ..Config::default()
    };
    println!("Custom Config: {:?}", custom);
}

fn demo_std_macros() {
    println!("Стандартні макроси зі std");

    let s = format!("Hello, {}! You are {} years old.", "Alice", 30);
    println!("format!: {}", s);

    fn _not_yet_done() -> i32 {
        todo!("implement this later")
    }

    let x = 5u32;
    println!("matches!(x, 1..=10): {}", matches!(x, 1..=10));
    println!("matches!(x, 11..=20): {}", matches!(x, 11..=20));

    let a = 2u32;
    let b = dbg!(a * 2) + 1;
    println!("b = {}", b);

    let greeting = concat!("Hello", ", ", "World", "!");
    println!("concat!: {}", greeting);

    let pkg = env!("CARGO_PKG_NAME");
    println!("Package name: {}", pkg);
}

fn demo_when_not_to_use() {
    println!("Коли НЕ варто використовувати макроси");

    macro_rules! add_bad {
        ($a:expr, $b:expr) => { $a + $b };
    }

    fn add_good(a: i32, b: i32) -> i32 { a + b }

    println!("add_bad!(2,3) = {0}, add_good(2,3) = {1}", add_bad!(2, 3), add_good(2, 3));
}

fn main() {
    println!("=== Завдання 3: Макроси у Rust ===\n");
    demo_declarative_macros();
    println!();
    demo_derive_macros();  // ← додай цей рядок
    println!();
    demo_std_macros();
    println!();
    demo_when_not_to_use();
}