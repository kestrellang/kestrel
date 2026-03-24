// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.i64 {
    { (x) in
        var acc = 0;
        acc = lang.i64_add(acc, x);
        acc = lang.i64_add(acc, x);
        acc
    }
}
