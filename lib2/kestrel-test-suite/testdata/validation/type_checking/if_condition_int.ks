// test: diagnostics
// stdlib: true

module Main

func test() {
    if 42 { // ERROR
        let x: lang.i64 = 1;
    }
}
