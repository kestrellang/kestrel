// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md, "How it is captured": a view is a live reference,
// not a one-time read — every call re-reads the frame slot. Pins that the
// closure tracks a whole sequence of writes rather than latching the value
// seen at the first call.
module Test

@main
func main() -> lang.i64 {
    var x = 1;
    let f = { x };
    if lang.i64_eq(f(), 1) { } else { return 1; }
    x = 2;
    if lang.i64_eq(f(), 2) { } else { return 2; }
    x = 3;
    if lang.i64_eq(f(), 3) { } else { return 3; }
    0
}
