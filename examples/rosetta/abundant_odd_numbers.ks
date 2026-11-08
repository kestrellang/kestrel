// Rosetta Code: Abundant odd numbers
// https://rosettacode.org/wiki/Abundant_odd_numbers
module Main

func divisorSum(n: Int64) -> Int64 {
    var sum: Int64 = 1;
    var d: Int64 = 3;
    while d * d <= n {
        if n % d == 0 {
            sum = sum + d;
            if d * d != n { sum = sum + n / d; }
        }
        d = d + 2;   // odd numbers have only odd divisors
    }
    sum
}

@main
func main() {
    var n: Int64 = 1;
    var found: Int64 = 0;
    println("First 25 abundant odd numbers:");
    while found < 25 {
        n = n + 2;
        let s = divisorSum(n);
        if s > n {
            found = found + 1;
            println("\(n:>6) divisor sum \(s)");
        }
    }
    while found < 1000 {
        n = n + 2;
        if divisorSum(n) > n { found = found + 1; }
    }
    println("1000th: \(n) with divisor sum \(divisorSum(n))");
    n = 1000000001;
    while divisorSum(n) <= n { n = n + 2; }
    println("First above one billion: \(n) with divisor sum \(divisorSum(n))");
}
