mod part_1;
mod part_2;

fn main() {
    println!("--- PART 1: Typestate Pattern ---");
    use part_1::Post;

    let post = Post::new("Hello World");
    let unmoderated = post.publish();
    let published = unmoderated.allow();
    let _deleted = published.delete();

    println!("Cycle complete.\n");

    println!("--- PART 2: JSON to TOML ---");
    part_2::run();
}