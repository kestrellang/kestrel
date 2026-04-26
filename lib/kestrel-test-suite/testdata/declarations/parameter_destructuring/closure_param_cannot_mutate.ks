// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let f = { ((a, b): (lang.i64, lang.i64)) in
        a = 10;  // ERROR: immutable
        a
    };
    f((1, 2))
}
