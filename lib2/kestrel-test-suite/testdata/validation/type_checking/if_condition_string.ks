// test: diagnostics
// stdlib: false

module Main

func test() {
    if "hello" { // ERROR
        let x: lang.i64 = 1;
    }
}
