// Rosetta Code: Factorial
// https://rosettacode.org/wiki/Factorial
module Main

func factorial(n: Int64) -> Int64 {
    (1..=n).iter().product()
}

func factorialRecursive(n: Int64) -> Int64 {
    if n <= 1 { 1 } else { n * factorialRecursive(n - 1) }
}

@main
func main() {
    for n in 0..=20 {
        println("\(n)! = \(factorial(n))");
    }
    println("recursive 15! = \(factorialRecursive(15))");
}
