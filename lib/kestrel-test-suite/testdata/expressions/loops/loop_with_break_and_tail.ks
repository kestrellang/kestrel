// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    loop {
        x = lang.i64_add(x, 1);
        if lang.i64_signed_gt(x, 5) {
            break;
        }
    }
    x
}
