// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    return lang.i64_add(1, 2);
    return 3; // WARN: unreachable
}
