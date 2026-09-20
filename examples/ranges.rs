// v0.8.0 的整數 range、inclusive endpoint、break 與 continue 範例。
fn main() {
    let mut total = 0;

    for value in 1..5 {
        if value == 2 {
            continue;
        }
        total += value;
    }

    for index in 0usize..=2usize {
        println!("index = {}", index);
    }

    println!();
    println!("total = {}", total);
}
