fn main() {
    let greeting = "你好";
    let suffix: &str = "!";
    let mut message = String::from(greeting);
    println!("{}{}", message, suffix);

    message = String::from("IdwC");
    let moved = message;
    println!("{}", moved);
}
