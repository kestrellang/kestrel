// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let ((a, b), c) = ((1, 2), 3);
    lang.i64_add(lang.i64_add(a, b), c)
}
