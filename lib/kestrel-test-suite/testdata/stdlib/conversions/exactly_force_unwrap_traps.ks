// test: execution
// stdlib: true
// expect-exit: -1

module Test

// There is no narrowing `Int8(from: Int64)`. To trap when a value doesn't
// fit, force-unwrap `exactly:`; it never silently truncates 300 to 44.
@main
func main() -> lang.i64 {
    let big: Int64 = 300;
    let small = Int8(exactly: big)!;
    if small == 44 { return 1 }
    0
}
