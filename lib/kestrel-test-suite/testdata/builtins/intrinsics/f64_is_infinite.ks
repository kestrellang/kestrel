// test: diagnostics
// stdlib: false

module Test
func isInfinite(f: lang.f64) -> lang.i1 {
    lang.f64_is_infinite(f)
}
