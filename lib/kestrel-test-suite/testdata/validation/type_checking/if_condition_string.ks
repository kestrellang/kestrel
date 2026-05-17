// test: diagnostics
// stdlib: true

module Main

func test() {
    if "hello" { // ERROR
        let x: lang.i64 = 1;
    }
}
