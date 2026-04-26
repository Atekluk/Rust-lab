use derive_more::{Display, From, Into, TryFrom};

#[derive(Debug, From, Into, Display)]
struct Meters(f64);

#[derive(Debug, From, Into, Display)]
struct Kilograms(f64);

fn demo_from_into() {
    println!("--- From / Into ---");

    let s = String::from("hello");
    let n: i64 = 42i32.into();
    println!("String::from: {}", s);
    println!("i32 into i64: {}", n);

    let m = Meters::from(100.0);
    let val: f64 = m.into();
    println!("Meters::from(100.0) -> into f64: {}", val);

    let kg = Kilograms(75.5);
    println!("Kilograms display: {}", kg);
}

#[derive(Debug, TryFrom)]
#[try_from(repr)]
#[repr(u8)]
enum Direction {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
}

fn demo_tryfrom() {
    println!("--- TryFrom / TryInto ---");

    let big: i64 = 1000;
    match i8::try_from(big) {
        Ok(v) => println!("i8::try_from(1000) = {}", v),
        Err(e) => println!("i8::try_from(1000) failed: {}", e),
    }

    let small: i64 = 42;
    match i8::try_from(small) {
        Ok(v) => println!("i8::try_from(42) = {}", v),
        Err(e) => println!("i8::try_from(42) failed: {}", e),
    }

    match Direction::try_from(2u8) {
        Ok(d) => println!("Direction::try_from(2) = {:?}", d),
        Err(e) => println!("Direction::try_from(2) failed: {}", e),
    }
    match Direction::try_from(99u8) {
        Ok(d) => println!("Direction::try_from(99) = {:?}", d),
        Err(e) => println!("Direction::try_from(99) failed: {}", e),
    }
}

fn print_length<S: AsRef<str>>(s: S) {
    println!("Length of '{}': {}", s.as_ref(), s.as_ref().len());
}

fn append_world(s: &mut String) {
    s.push_str(", world");
}

fn demo_asref_asmut() {
    println!("--- AsRef / AsMut ---");

    print_length("hello");
    print_length(String::from("goodbye"));

    let mut s = String::from("hello");
    append_world(&mut s);
    println!("After AsMut push: {}", s);
}

use std::collections::HashMap;

fn demo_borrow() {
    println!("--- Borrow ---");
    let mut map: HashMap<String, i32> = HashMap::new();
    map.insert(String::from("one"), 1);
    map.insert(String::from("two"), 2);

    println!("map[\"one\"] = {:?}", map.get("one"));
    println!("map[\"two\"] = {:?}", map.get("two"));
    println!("map[\"three\"] = {:?}", map.get("three"));
}

fn takes_str(s: &str) {
    println!("takes_str received: {}", s);
}

fn demo_deref() {
    println!("--- Deref coercion ---");

    let boxed = Box::new(String::from("hello"));
    takes_str(&boxed); 

    let v = vec![1, 2, 3, 4, 5];
    let sum: i32 = v.iter().sum();
    println!("Vec deref to slice, sum = {}", sum);

    let mut boxed_val = Box::new(42);
    *boxed_val += 1; 
    println!("Box after DerefMut: {}", boxed_val);
}

fn main() {
    println!("Перетворення між типами ===\n");

    demo_from_into();
    println!();
    demo_tryfrom();
    println!();
    demo_asref_asmut();
    println!();
    demo_borrow();
    println!();
    demo_deref();
}