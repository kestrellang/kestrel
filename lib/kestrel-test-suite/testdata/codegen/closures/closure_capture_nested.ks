// test: diagnostics
// stdlib: true

module Test

func main() -> lang.i64 {
    let outer: std.num.Int64 = 20;
    let make_inner = { (x: std.num.Int64) in { x + outer } }; // ERROR: cannot return a closure that captures variables
    let inner = make_inner(22);
    if inner() != 42 { return 1 }
    0
}
