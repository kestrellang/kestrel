// test: diagnostics
// stdlib: false

module Main

func test() {
    let f: (lang.i64, lang.i64) -> lang.i64 = { (a, b) in lang.i64_add(a, b) };
}
