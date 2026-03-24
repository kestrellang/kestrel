// test: diagnostics
// stdlib: false

module Test

func test() -> lang.i64 {
    let f: (lang.i64) -> lang.i64 = { lang.i64_mul(it, 2) };
    f(21)
}
