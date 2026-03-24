// test: diagnostics
// stdlib: false

module Main

func test() {
    while 42 { // ERROR
        let x: lang.i64 = 1;
    }
}
