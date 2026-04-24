// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.i64 {
    { (x) in
        x = 10; // ERROR: cannot assign
        x
    }
}
