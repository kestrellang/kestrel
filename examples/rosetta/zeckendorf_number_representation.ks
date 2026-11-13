// Rosetta Code: Zeckendorf number representation
// https://rosettacode.org/wiki/Zeckendorf_number_representation
module Main

func zeckendorf(n: Int64) -> String {
    if n == 0 { return "0" }
    var fibs: Array[Int64] = [1, 2];
    while fibs(fibs.count - 1) <= n {
        fibs.append(fibs(fibs.count - 1) + fibs(fibs.count - 2));
    }
    // Greedily take the largest Fibonacci number that fits.
    var rest = n;
    var digits = StringBuilder();
    var started = false;
    var i = fibs.count - 1;
    while i >= 0 {
        if fibs(i) <= rest {
            digits.append("1");
            rest = rest - fibs(i);
            started = true;
        } else if started {
            digits.append("0");
        }
        i = i - 1;
    }
    digits.build()
}

@main
func main() {
    for n in 0..=20 {
        println("\(n:>2): \(zeckendorf(n))");
    }
}
