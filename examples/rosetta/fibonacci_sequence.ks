// Rosetta Code: Fibonacci sequence
// https://rosettacode.org/wiki/Fibonacci_sequence
module Main

// Iterative: O(n) time, O(1) space.
func fib(n: Int64) -> Int64 {
    var a: Int64 = 0;
    var b: Int64 = 1;
    for _ in 0..<n {
        let next = a + b;
        a = b;
        b = next;
    }
    a
}

// Recursive, straight from the definition.
func fibRecursive(n: Int64) -> Int64 {
    if n < 2 { n } else { fibRecursive(n - 1) + fibRecursive(n - 2) }
}

@main
func main() {
    let first = (0..<20).iter().map { fib(it) }.collect();
    println("First 20: \(first)");
    println("fib(90) = \(fib(90))");
    println("fibRecursive(25) = \(fibRecursive(25))");
}
