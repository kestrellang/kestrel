// Rosetta Code: Ethiopian multiplication
// https://rosettacode.org/wiki/Ethiopian_multiplication
module Main

func halve(n: Int64) -> Int64 { n / 2 }
func double(n: Int64) -> Int64 { n * 2 }
func isEven(n: Int64) -> Bool { n % 2 == 0 }

func ethiopian(a: Int64, b: Int64) -> Int64 {
    var left = a;
    var right = b;
    var total = 0;
    while left >= 1 {
        let kept = if isEven(left) { "STRUCK" } else { "KEPT" };
        println("\(left:>4) \(right:>6) \(kept)");
        if not isEven(left) { total = total + right; }
        left = halve(left);
        right = double(right);
    }
    total
}

@main
func main() {
    println("17 x 34 = \(ethiopian(17, 34))");
}
