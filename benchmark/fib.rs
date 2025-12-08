use std::time::Instant;

fn fib(n: u64) -> u64 {
    if n < 2 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    println!("--- RUST BENCHMARK (Fib 30) ---");
    
    let start = Instant::now();
    let result = fib(30);
    let duration = start.elapsed();

    println!("Resultat : {}", result);
    println!("Temps : {} ms", duration.as_millis());
}