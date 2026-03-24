// test: diagnostics
// stdlib: false

module Main

func test() {
    let t = (1, 2);
    t.0 = 10; // ERROR: cannot assign
}
