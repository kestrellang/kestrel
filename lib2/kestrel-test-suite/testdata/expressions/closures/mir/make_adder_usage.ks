// test: diagnostics
// stdlib: false

module Main

func makeAdder(n: lang.i64) -> (lang.i64) -> lang.i64 {
    { (x: lang.i64) in lang.i64_add(x, n) } // ERROR: cannot return a closure that captures variables
}

func main() -> lang.i64 {
    let add5 = makeAdder(5);
    let add10 = makeAdder(10);

    lang.i64_add(add5(3), add10(3))
}
