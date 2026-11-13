// Rosetta Code: Semiprime
// https://rosettacode.org/wiki/Semiprime
module Main

func isSemiprime(n: Int64) -> Bool {
    var rest = n;
    var factors = 0;
    var d = 2;
    while d * d <= rest and factors < 2 {
        while rest % d == 0 {
            rest = rest / d;
            factors = factors + 1;
        }
        d = d + 1;
    }
    if rest > 1 { factors = factors + 1; }
    factors == 2
}

@main
func main() {
    println((1..=100).iter().filter { isSemiprime(it) }.collect());
    println("1679 (radio message): \(isSemiprime(1679))");
}
