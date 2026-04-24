// test: diagnostics
// stdlib: false

module Main

func apply(f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(5)
}

func test() -> (lang.i64) -> lang.i64 {
    {
        let outer = it;
        apply({ lang.i64_add(it, outer) })
    }
}
