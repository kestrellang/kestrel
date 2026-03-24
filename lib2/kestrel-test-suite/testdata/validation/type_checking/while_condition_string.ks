// test: diagnostics
// stdlib: false

module Main

func test() {
    while "hello" { // ERROR
        let x: lang.i64 = 1;
    }
}
