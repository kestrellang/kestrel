// test: diagnostics
// stdlib: false

module Main

func test() {
    var counter: lang.i64 = 0;
    while lang.i64_signed_lt(counter, 10) {
        counter = lang.i64_add(counter, 1);
    }
}
