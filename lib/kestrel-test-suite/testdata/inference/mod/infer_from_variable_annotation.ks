// test: diagnostics
// stdlib: false

module Main

func test() {
    let x: lang.i64 = 42;
    let y = lang.i64_add(x, 1);
    let z: lang.i64 = y;
}
