// test: diagnostics
// stdlib: false

module Main

func test() {
    let x: lang.i64 = 1;
    continue; // ERROR: outside of loop
    let y: lang.i64 = 2;
}
