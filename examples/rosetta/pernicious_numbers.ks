// Rosetta Code: Pernicious numbers
// https://rosettacode.org/wiki/Pernicious_numbers
module Main

func isPrime(n: Int64) -> Bool {
    if n < 2 { return false }
    var d = 2;
    while d * d <= n {
        if n % d == 0 { return false }
        d = d + 1;
    }
    true
}

func isPernicious(n: Int64) -> Bool {
    isPrime(UInt64(truncating: n).countOnes)
}

@main
func main() {
    println((1..).iter().filter { isPernicious(it) }.take(25).collect());
    println((888888877..=888888888).iter().filter { isPernicious(it) }.collect());
}
