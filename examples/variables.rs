fn main() {
    let mut total: i32 = 6;
    total += 3 * 4;
    let disabled = false;
    let ready: bool = total >= 18 && !disabled;
    println!("total = {}, ready = {}", total, ready);

    {
        let total = total / 2;
        println!("inner total = {}", total);
    }

    total = 17;
    println!("outer total = {}", total);
}
