// test: diagnostics
// stdlib: true

module Test

func main() -> lang.i64 {
    let outer: std.numeric.Int64 = 20;
    let make_inner = { (x: std.numeric.Int64) in { x + outer } }; // ERROR(E494)
    let inner = make_inner(22);
    if inner() != 42 { return 1 }
    0
}
