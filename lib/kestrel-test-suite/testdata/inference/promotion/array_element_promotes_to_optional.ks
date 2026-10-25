// test: execution
// stdlib: true
// expect-exit: 0

// bidi ruling D7(i): array elements checked against `[Int64?]` promote each
// bare element, like an annotated `let` does. Before P3 the elements were
// equated, so `1` and `3` were rejected ("expected Optional[Int64] got Int64").

module Test

@main
func main() -> lang.i64 {
    let xs: [Int64?] = [1, .None, 3];
    if xs.count != 3 { return 1; }
    if xs(unchecked: 0) != .Some(1) { return 2; }
    if xs(unchecked: 1) != .None { return 3; }
    0
}
