// v0.7.0 限定的行輸入、token 解析與 powi(2) 範例。
fn main() {
    print!("Enter weight (kg) and height (m): ");
    idwc::io::flush_stdout();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let values: Vec<&str> = input.split_whitespace().collect();
    let weight = values[0].parse::<f64>().unwrap();
    let height = values[1].parse::<f64>().unwrap();
    let bmi = weight / height.powi(2);

    println!("BMI = {:.2}, below 25 = {}", bmi, bmi < 25.0);
}
