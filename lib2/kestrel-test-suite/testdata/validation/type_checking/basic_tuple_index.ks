// test: diagnostics
// stdlib: false

module Main

func test() {
    let t = (1, "hello", true);
    let x: lang.i64 = t.0;
    let y: lang.str = t.1;
    let z: lang.i1 = t.2;
}
