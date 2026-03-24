// test: diagnostics
// stdlib: false

module Test
func extract(cf: Prelude.ControlFlow[lang.i64, lang.str]) -> lang.i64 {
    match cf {
        .Continue(value) => value,
        .Break(_msg) => 0
    }
}
