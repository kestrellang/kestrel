// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    let x = if cond { 1 } else { 2 };
    x
}
