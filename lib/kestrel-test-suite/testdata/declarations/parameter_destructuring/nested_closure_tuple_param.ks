// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let nested = { (((a, b), c): ((lang.i64, lang.i64), lang.i64)) in
        lang.i64_add(lang.i64_add(a, b), c)
    };
    nested(((1, 2), 3))
}
