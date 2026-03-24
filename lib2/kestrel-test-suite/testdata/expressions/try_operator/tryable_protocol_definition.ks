// test: diagnostics
// stdlib: false

module Test
func test() {
    let _: Prelude.ControlFlow[lang.i64, lang.str] = Prelude.ControlFlow.Continue(42);
}
