// test: diagnostics
// stdlib: false

// A closure's implicit `it` never captures an outer binding named `it`: the
// closure uses its own parameter, and hiding the outer binding is E143.

module Main

func apply(f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(5)
}

func test() -> lang.i64 {
    let it = 100;
    apply({ lang.i64_add(it, 1) }) // WARNING: implicit parameter 'it' shadows the outer binding 'it'
}
