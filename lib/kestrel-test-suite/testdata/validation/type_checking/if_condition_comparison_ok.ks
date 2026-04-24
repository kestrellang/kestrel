// test: diagnostics
// stdlib: false

module Main

func test() {
    let x: lang.i64 = 5;
    if lang.i64_eq(x, 5) {
        let y: lang.i64 = 1;
    }
}
