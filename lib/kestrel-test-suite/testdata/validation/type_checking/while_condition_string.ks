// test: diagnostics
// stdlib: true

module Main

func test() {
    while "hello" { // ERROR
        let x: lang.i64 = 1;
    }
}
