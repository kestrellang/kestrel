// test: diagnostics
// stdlib: false

module Main

func test() {
    loop {
        break;
        let x: lang.i64 = 1; // WARN: unreachable
    }
}
