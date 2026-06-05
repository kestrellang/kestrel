// test: execution
// stdlib: true
// expect-exit: 0

module Test

@main
func main() -> lang.i64 {
    // Pound escalation lets the body contain `"#`.
    let s = ##"""
        contains "# inside
        """##;
    if s != "contains \"# inside" { return 1 }
    0
}
