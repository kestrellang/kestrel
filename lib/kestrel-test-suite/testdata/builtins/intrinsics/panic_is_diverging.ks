// test: diagnostics
// stdlib: false

module Test
func unreachable() -> lang.i64 {
    lang.panic_unwind("unreachable");
}
