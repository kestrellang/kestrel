// test: diagnostics
// stdlib: false

module Main

func test() {
    let outer: lang.i64 = 10;
    while lang.i64_signed_gt(outer, 0) {
        break;
    }
}
