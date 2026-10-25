// test: execution
// stdlib: true
// expect-exit: 0

// bidi ruling D2 / P2 operator rule: the two operands of a homogeneous
// arithmetic or comparison operator unify while both are still open literals
// (Rust's built-in binop rule), so `1 + 2.5` is one Float64 rather than an
// Int64 receiver met by a Float64 argument ("expected Int64 got Float64").

module Test

@main
func main() -> lang.i64 {
    let a = 1 + 2.5;
    if a != 3.5 { return 1; }
    let b = 2.5 * 2;
    if b != 5.0 { return 2; }
    if (1 < 2.5) == false { return 3; }
    let c = 7 - 2;
    if c != 5 { return 4; }
    0
}
