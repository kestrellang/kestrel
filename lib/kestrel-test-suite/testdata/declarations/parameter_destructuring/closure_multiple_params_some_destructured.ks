// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let f = { (x: lang.i64, (a, b): (lang.i64, lang.i64)) in
        lang.i64_add(x, lang.i64_add(a, b))
    };
    f(10, (1, 2))
}
