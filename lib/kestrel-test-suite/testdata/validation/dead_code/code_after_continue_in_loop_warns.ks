// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
        continue;
        let y: lang.i64 = 2; // WARN: unreachable
    }
}
