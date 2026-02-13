fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
    return n;
}
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> () {
    println!("{}", fibonacci(0));
    println!("{}", fibonacci(1));
    println!("{}", fibonacci(5));
    println!("{}", fibonacci(10));
}