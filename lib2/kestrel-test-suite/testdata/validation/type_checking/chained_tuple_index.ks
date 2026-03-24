// test: diagnostics
// stdlib: false

module Main

func test() {
    let t = ((1, 2), (3, 4));
    let inner = t.0;
    let x: lang.i64 = inner.1;
    let inner2 = t.1;
    let y: lang.i64 = inner2.0;
}
