// test: diagnostics
// stdlib: false

// The inner closure's `it` is its own parameter (innermost closure wins);
// hiding the outer closure's `it` is warned about (E142).

module Main

func apply(f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(10)
}

func test() -> (lang.i64) -> lang.i64 {
    {
        let outer = it;
        apply({ lang.i64_add(it, outer) }) // WARNING: implicit parameter 'it' shadows the 'it' of an enclosing closure
    }
}
