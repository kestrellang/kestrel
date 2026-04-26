// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var i = 0;
    loop {
        i = lang.i64_add(i, 1);
        if lang.i64_signed_ge(i, 10) {
            break
        }
    }
    i
}
