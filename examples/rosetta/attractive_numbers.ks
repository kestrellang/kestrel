// Rosetta Code: Attractive numbers
// https://rosettacode.org/wiki/Attractive_numbers
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

func primeFactorCount(n: Int64) -> Int64 {
    var rest = n;
    var count = 0;
    var d = 2;
    while d * d <= rest {
        while rest % d == 0 { rest = rest / d; count = count + 1; }
        d = d + 1;
    }
    if rest > 1 { count + 1 } else { count }
}

@main
func main() {
    let attractive = (1..=120).iter().filter { isPrime(primeFactorCount(it)) }.collect();
    println("Attractive numbers up to 120:");
    for row in attractive.chunks(of: 20).toArray() {
        println(row.map { "\(it:>4)" }.joined(by: ""));
    }
}
