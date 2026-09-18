fn main() {
    print!("Enter two integers: ");
    idwc::io::flush_stdout();
    let first = idwc::io::read_i32();
    let second = idwc::io::read_i32();
    let total = sum(first, second);
    println!("sum = {}, positive = {}", total, positive(total));
}

fn sum(first: i32, second: i32) -> i32 {
    first + second
}

fn positive(value: i32) -> bool {
    if value > 0 {
        return true;
    }
    false
}
