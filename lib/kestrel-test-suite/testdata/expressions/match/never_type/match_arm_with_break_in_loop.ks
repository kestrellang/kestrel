// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var result = 0;
    loop {
        match true {
            true => break,
            false => { result = 1; }
        }
    }
    result
}
