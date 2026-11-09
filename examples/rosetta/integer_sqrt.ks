// Rosetta Code: Isqrt (integer square root) of X
// https://rosettacode.org/wiki/Isqrt_(integer_square_root)_of_X
module Main

// The quadratic residue method from the task, all in integers.
func isqrt(x: Int64) -> Int64 {
    var q = 1;
    while q <= x { q = q << 2; }
    var z = x;
    var r = 0;
    while q > 1 {
        q = q >> 2;
        let t = z - r - q;
        r = r >> 1;
        if t >= 0 {
            z = t;
            r = r + q;
        }
    }
    r
}

@main
func main() {
    println("isqrt of 0..65:");
    println((0..=65).iter().map { isqrt(it) }.collect().joined(" "));
    println("isqrt of odd powers of 7:");
    var p = 7;
    for k in 0..<11 {
        println("7^\(2 * k + 1:<2) = \(p:>20)   isqrt = \(isqrt(p):>11)");
        if k < 10 { p = p * 49; }
    }
}
