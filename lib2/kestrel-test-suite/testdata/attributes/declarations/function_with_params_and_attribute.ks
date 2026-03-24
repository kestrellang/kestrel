// test: diagnostics
// stdlib: false

module Test
@dummy
func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_add(a, b)
}
