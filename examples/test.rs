fn main() {
    let mut buf: String = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    let list: Vec<&str> = buf.split_whitespace().collect();

    let kg: f64 = list[0].parse().unwrap();
    let cm: f64 = list[1].parse().unwrap();

    let bmi: f64 = kg / (cm * 0.01).powi(2);

    if bmi < 18.5 {
        println!("過瘦");
    } else if bmi < 24.0 {
        println!("標準");
    } else if bmi < 27.0 {
        println!("過重");
    } else if bmi < 30.0 {
        println!("輕度肥胖");
    } else if bmi < 35.0 {
        println!("中度肥派");
    } else {
        println!("重度肥胖");
    }
}
