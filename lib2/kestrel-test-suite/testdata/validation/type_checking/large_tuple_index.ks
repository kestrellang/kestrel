// test: diagnostics
// stdlib: false

module Main

func test() {
    let t = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
    let x: lang.i64 = t.9;
}
