// Rosetta Code: Accumulator factory
// https://rosettacode.org/wiki/Accumulator_factory
module Main

// The returned closure owns its running total; each call adds to it.
func foo(start: Float64) -> escaping (Float64) -> Float64 {
    var sum = start;
    { (n: Float64) in
        sum = sum + n;
        sum
    }
}

@main
func main() {
    let x = foo(1.0);
    x(5.0);
    foo(3.0);
    println(x(2.3));
}
