// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let add = { ((a, b): (lang.i64, lang.i64)) in lang.i64_add(a, b) };
    add((1, 2))
}
