// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    loop {
        if lang.i64_eq(x, 10) {
            return x
        }
        x = lang.i64_add(x, 1);
    }
}
