// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let f = { ((a, b): (lang.i64, lang.i64)) in
        // a = 10;  // Would be error: closure params are immutable
        lang.i64_add(a, b)
    };
    f((1, 2))
}
