// test: diagnostics
// stdlib: false

module Main

func test() {
    let t = ((1, 2), (3, 4));
    let inner = t.0;
    let x: lang.i64 = inner.0;
}
