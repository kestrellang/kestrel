// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    match x {
        var n => {
            n = lang.i64_add(n, 1);
            n
        }
    }
}
