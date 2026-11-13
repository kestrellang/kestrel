// Rosetta Code: Factors of an integer
// https://rosettacode.org/wiki/Factors_of_an_integer
module Main

func factors(n: Int64) -> Array[Int64] {
    var small = Array[Int64]();
    var large = Array[Int64]();
    var d = 1;
    while d * d <= n {
        if n % d == 0 {
            small.append(d);
            if d * d != n { large.insert(n / d, at: 0); }
        }
        d = d + 1;
    }
    small.append(contentsOf: large);
    small
}

@main
func main() {
    for n in [1, 45, 53, 64, 360, 1000] {
        println("\(n): \(factors(n))");
    }
}
