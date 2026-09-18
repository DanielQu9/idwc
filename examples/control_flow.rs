fn main() {
    let mut number = 0;
    let mut total = 0;

    while number < 6 {
        number += 1;
        if number % 2 == 0 {
            continue;
        }
        total += number;
    }

    if total > 9 {
        println!("large total: {}", total);
    } else if total == 9 {
        println!("total = {}", total);
    } else {
        println!("small total: {}", total);
    }

    loop {
        total -= 1;
        if total == 6 {
            break;
        }
    }
    println!("after loop = {}", total);
}
