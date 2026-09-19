fn main() {
    let kg: f64 = idwc::io::read_f64();
    let cm: f64 = idwc::io::read_f64();

    let bmi: f64 = kg / ((cm * 0.01) * (cm * 0.01));

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
