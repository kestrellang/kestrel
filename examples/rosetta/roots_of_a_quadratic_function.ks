// Rosetta Code: Roots of a quadratic function
// https://rosettacode.org/wiki/Roots_of_a_quadratic_function
module Main

// Avoids cancellation by computing the larger root first, then using
// x1 * x2 = c / a for the other.
func roots(a: Float64, b: Float64, c: Float64) -> String {
    let d = b * b - 4.0 * a * c;
    if d < 0.0 {
        let re = -b / (2.0 * a);
        let im = (-d).sqrt() / (2.0 * a);
        return "\(re) +/- \(im)i"
    }
    let q = -0.5 * (b + b.sign * d.sqrt());
    let x1 = q / a;
    let x2 = c / q;
    "\(x1), \(x2)"
}

@main
func main() {
    println(roots(1.0, -3.0, 2.0));
    println(roots(1.0, 2.0, 5.0));
    println(roots(1.0, -1.0e9, 1.0));
    println(roots(1.0, 2.0, 1.0));
}
