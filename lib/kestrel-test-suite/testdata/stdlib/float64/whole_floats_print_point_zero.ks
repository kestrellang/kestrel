// test: execution
// stdlib: true
// expect-stdout: 3.0 0.0 -0.0 -2.0 100000.0 2.5 4.0\n1e20 3 3.00 [   3.0]\n

// A whole float prints with ".0", so it never reads as an integer. An
// explicit precision still means exactly that many decimals, and large
// values still switch to scientific notation.
module Test

@main
func main() -> Int64 {
    let a: Float64 = 3.0;
    let z: Float64 = 0.0;
    let nz: Float64 = -0.0;
    let n: Float64 = -2.0;
    let big: Float64 = 100000.0;
    let h: Float64 = 2.5;
    let f: Float32 = 4.0;
    let huge: Float64 = 1.0e20;
    println("\(a) \(z) \(nz) \(n) \(big) \(h) \(f)");
    println("\(huge) \(a:.0) \(a:.2) [\(a:>6)]");
    0
}
