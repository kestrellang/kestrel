// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    loop {
        x = lang.i64_add(x, 1);
        if lang.i64_signed_gt(x, 10) {
            break;
        }
        continue;
    }
}
