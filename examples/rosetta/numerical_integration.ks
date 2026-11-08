// Rosetta Code: Numerical integration
// https://rosettacode.org/wiki/Numerical_integration
module Main

// Sums f(a + (i + offset) * h) for i in first..<last.
func sample(f: (Float64) -> Float64, a: Float64, h: Float64, first: Int64, last: Int64, offset: Float64) -> Float64 {
    var sum = 0.0;
    for i in first..<last { sum = sum + f(a + (Float64(from: i) + offset) * h); }
    sum
}

func leftRect(f: (Float64) -> Float64, a: Float64, b: Float64, n: Int64) -> Float64 {
    let h = (b - a) / Float64(from: n);
    h * sample(f, a, h, 0, n, 0.0)
}

func rightRect(f: (Float64) -> Float64, a: Float64, b: Float64, n: Int64) -> Float64 {
    let h = (b - a) / Float64(from: n);
    h * sample(f, a, h, 1, n + 1, 0.0)
}

func midRect(f: (Float64) -> Float64, a: Float64, b: Float64, n: Int64) -> Float64 {
    let h = (b - a) / Float64(from: n);
    h * sample(f, a, h, 0, n, 0.5)
}

func trapezium(f: (Float64) -> Float64, a: Float64, b: Float64, n: Int64) -> Float64 {
    let h = (b - a) / Float64(from: n);
    h * ((f(a) + f(b)) / 2.0 + sample(f, a, h, 1, n, 0.0))
}

func simpson(f: (Float64) -> Float64, a: Float64, b: Float64, n: Int64) -> Float64 {
    let h = (b - a) / Float64(from: n);
    let mid = sample(f, a, h, 0, n, 0.5);
    let inner = sample(f, a, h, 1, n, 0.0);
    h / 6.0 * (f(a) + f(b) + 4.0 * mid + 2.0 * inner)
}

func report(name: String, f: (Float64) -> Float64, a: Float64, b: Float64, n: Int64) {
    println("\(name) on [\(a), \(b)] with \(n) steps:");
    println("  left \(leftRect(f, a, b, n):.6)  right \(rightRect(f, a, b, n):.6)  mid \(midRect(f, a, b, n):.6)  trapezium \(trapezium(f, a, b, n):.6)  simpson \(simpson(f, a, b, n):.6)");
}

@main
func main() {
    report("x^3", { it * it * it }, 0.0, 1.0, 100);
    report("1/x", { 1.0 / it }, 1.0, 100.0, 1000);
    report("x", { it }, 0.0, 5000.0, 5000000);
    report("x", { it }, 0.0, 6000.0, 6000000);
}
