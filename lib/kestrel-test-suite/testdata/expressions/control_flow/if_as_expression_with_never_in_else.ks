// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    let x: lang.i64 = if cond { 42 } else { return 0 };
    x
}
