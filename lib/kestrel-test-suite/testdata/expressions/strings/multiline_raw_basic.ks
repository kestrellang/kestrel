// test: execution
// stdlib: true
// expect-exit: 0

module Test

func main() -> lang.i64 {
    // Multi-line raw: indent strip applies but no escape decoding.
    let s = #"""
        a\nb
        c\td
        """#;
    if s != "a\\nb\nc\\td" { return 1 }
    0
}
