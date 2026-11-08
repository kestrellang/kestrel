// Rosetta Code: Catalan numbers
// https://rosettacode.org/wiki/Catalan_numbers
module Main

// C(n+1) = C(n) * 2(2n+1) / (n+2)
func catalan(n: Int64) -> Int64 {
    var c: Int64 = 1;
    for i in 0..<n {
        c = c * 2 * (2 * i + 1) / (i + 2);
    }
    c
}

@main
func main() {
    for n in 0..=15 {
        println("\(n:>2) \(catalan(n))");
    }
}
