// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64, lang.i64) -> lang.i64 {
    { (x, y) in
        let sum = lang.i64_add(x, y);
        let doubled = lang.i64_mul(sum, 2);
        let result = lang.i64_add(doubled, 1);
        result
    }
}
