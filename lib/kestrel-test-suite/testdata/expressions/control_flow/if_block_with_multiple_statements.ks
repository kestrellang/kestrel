// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    if true {
        let a: lang.i64 = 1;
        let b: lang.i64 = 2;
        let c: lang.i64 = lang.i64_add(a, b);
        c
    } else {
        0
    }
}
