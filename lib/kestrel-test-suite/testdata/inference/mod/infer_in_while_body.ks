// test: diagnostics
// stdlib: false

module Main

func test() {
    var count = 0;
    while lang.i64_signed_lt(count, 10) {
        let doubled = lang.i64_mul(count, 2);
        count = lang.i64_add(count, 1);
    }
}
