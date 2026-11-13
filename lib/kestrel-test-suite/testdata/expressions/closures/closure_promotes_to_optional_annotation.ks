// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A closure literal checked against an Optional function type is promoted,
// as any other value is.
@main
func main() -> lang.i64 {
    let f: (escaping (Int64) -> Int64)? = { (x: Int64) in x + 1 };
    let g = f!;
    if g(2) != 3 { return 1 }
    // `it` closures take their parameter type through the Optional too.
    let h: (escaping (Int64) -> Int64)? = { it * 2 };
    let k = h!;
    if k(4) != 8 { return 2 }
    0
}
