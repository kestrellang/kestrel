// test: diagnostics
// stdlib: false
// include: try_prelude.ks

module Test
func test() {
    let cont: Prelude.ControlFlow[lang.i64, lang.str] = Prelude.ControlFlow.Continue(42);
    let brk: Prelude.ControlFlow[lang.i64, lang.str] = Prelude.ControlFlow.Break("error");
}
