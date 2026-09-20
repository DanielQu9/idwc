fn main() {
    let mut values = vec![1, 2, 3];
    values.push(4);
    values[1] = 20;
    println!("{} {} {}", values.len(), values[1], values[3]);
    println!("values = {:?}", values);

    let repeated = vec![true; 3];
    println!("{} {}", repeated.len(), repeated[2]);
    println!("repeated = {:?}", repeated);

    let moved = values;
    println!("{}", moved[0]);

    let decimals: Vec<f64> = vec![1.5];
    println!("{}", decimals[0]);
}
