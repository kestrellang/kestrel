// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    if true {
        let x: lang.i64 = 5;
        let y: lang.i64 = lang.i64_add(x, 1);
        y
    } else {
        0
    }
}
