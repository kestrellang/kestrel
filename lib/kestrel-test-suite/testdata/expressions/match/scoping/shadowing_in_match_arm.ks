// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    let y = 100;
    match x {
        y => lang.i64_add(y, 1)
    }
}
