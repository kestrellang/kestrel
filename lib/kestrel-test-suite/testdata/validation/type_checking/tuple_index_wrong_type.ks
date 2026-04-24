// test: diagnostics
// stdlib: false

module Main

func test() {
    let t = (1, "hello");
    let x: lang.str = t.0; // ERROR
}
