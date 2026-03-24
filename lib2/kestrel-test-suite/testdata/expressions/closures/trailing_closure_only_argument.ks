// test: diagnostics
// stdlib: false

module Main

func apply(f: () -> lang.i64) -> lang.i64 {
    f()
}

func test() -> lang.i64 {
    apply { 42 }
}
