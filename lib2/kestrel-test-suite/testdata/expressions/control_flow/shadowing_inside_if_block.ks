// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let x: lang.i64 = 100;
    if true {
        let x: lang.i64 = 1;
        x
    } else {
        x
    }
}
