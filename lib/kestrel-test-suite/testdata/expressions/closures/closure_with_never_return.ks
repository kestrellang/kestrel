// test: diagnostics
// stdlib: false

module Main

func earlyReturn(f: () -> lang.i64) -> lang.i64 {
    f()
}

func test() -> lang.i64 {
    earlyReturn({
        return 42
    })
}
