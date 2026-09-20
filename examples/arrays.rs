// 固定長度陣列保留 Rust 的值複製語意，索引會在執行時檢查邊界。
fn main() {
    let mut scores: [i32; 4] = [12, 18, 9, 15];
    let original = scores;
    let mut index: usize = 0;

    while index < scores.len() {
        scores[index] += 1;
        index += 1;
    }

    println!(
        "first = {}, last = {}, original first = {}, length = {}",
        scores[0],
        scores[3],
        original[0],
        scores.len()
    );
    println!("scores = {:?}", scores);
}
