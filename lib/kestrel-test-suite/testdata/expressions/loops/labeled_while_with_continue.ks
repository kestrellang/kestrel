// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    outer: while lang.i64_signed_lt(x, 100) {
        x = lang.i64_add(x, 1);
        while true {
            continue outer;
        }
    }
}
