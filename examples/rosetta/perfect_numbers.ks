// Rosetta Code: Perfect numbers
// https://rosettacode.org/wiki/Perfect_numbers
module Main

func isPerfect(n: Int64) -> Bool {
    if n < 2 { return false }
    var sum = 1;
    var d = 2;
    while d * d <= n {
        if n % d == 0 {
            sum = sum + d;
            if d * d != n { sum = sum + n / d; }
        }
        d = d + 1;
    }
    sum == n
}

@main
func main() {
    println((1..<100000).iter().filter { isPerfect(it) }.collect());
    println("33550336 is perfect: \(isPerfect(33550336))");
}
