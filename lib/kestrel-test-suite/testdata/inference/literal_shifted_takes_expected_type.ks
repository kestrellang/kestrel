// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A shift's count is always Int64, so it says nothing about the value
// shifted: `1 << n` takes its type from the annotation, not from `n`.
@main
func main() -> lang.i64 {
    let n: Int64 = 3;
    let a: UInt32 = 1 << n;
    if a != 8 { return 1 }
    let b: UInt64 = 1 << 63;
    if b != 9223372036854775808 { return 2 }
    let c: UInt8 = 255 >> n;
    if c != 31 { return 3 }
    // With no annotation the literal defaults to Int64 as usual.
    let d = 1 << n;
    let e: Int64 = d;
    if e != 8 { return 4 }
    0
}
