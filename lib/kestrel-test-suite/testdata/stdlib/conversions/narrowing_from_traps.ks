// test: execution
// stdlib: true
// expect-exit: -1

module Test

// `Int8(from:)` traps when the value doesn't fit, instead of silently
// truncating 300 to 44.
@main
func main() -> lang.i64 {
    let big: Int64 = 300;
    let small = Int8(from: big);
    if small == 44 { return 1 }
    0
}
