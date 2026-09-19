// 用受限的型別化 stdin 示範浮點輸入、函式與固定精度輸出。
fn main() {
    print!("Enter weight (kg) and height (m): ");
    idwc::io::flush_stdout();
    let weight: f64 = idwc::io::read_f64();
    let height = idwc::io::read_f64();
    let result = bmi(weight, height);
    println!("BMI = {:.2}, below 25 = {}", result, result < 25.0);
}

// 輸入以公斤及公尺表示，不需要額外轉型或數學方法。
fn bmi(weight: f64, height: f64) -> f64 {
    weight / (height * height)
}
