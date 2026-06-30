// test: diagnostics
// stdlib: false

module Main

func makeAdder(n: lang.i64) -> (lang.i64) -> lang.i64 {
    { lang.i64_add(it, n) } // ERROR(E494)
}

func test() -> lang.i64 {
    let add5 = makeAdder(5);
    add5(10)
}
