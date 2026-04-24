// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.i64 {
    { (x) in
        let y = lang.i64_mul(x, 2);
        y
    }
}
