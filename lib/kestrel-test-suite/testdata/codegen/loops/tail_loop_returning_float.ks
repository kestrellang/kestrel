// test: execution
// stdlib: true
// expect-stdout: 12.0\n

// A Float64-returning function whose body ends in a `loop` that only exits by
// `return`. Today Cranelift (-O0) rejects the function: "result 0 has type
// i64, must match function signature of f64" — the loop's never-typed tail
// is lowered as an i64 zero.
// Found while writing examples/rosetta (Nth root via Newton's method).

module Test

func firstAbove(start: Float64, limit: Float64) -> Float64 {
    var x = start;
    loop {
        if x > limit { return x }
        x = x * 2.0;
    }
}

@main
func main() {
    println(firstAbove(1.5, 10.0));
}
