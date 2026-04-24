// test: diagnostics
// stdlib: false

module Main

func test() {
    loop {
        let y: lang.i64 = 10;
        break;
    }
    y // ERROR: undefined
}
