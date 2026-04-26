// test: diagnostics
// stdlib: false

module Main

protocol A: B { // ERROR: circular
    func a() -> lang.i64
}

protocol B: C {
    func b() -> lang.i64
}

protocol C: A {
    func c() -> lang.i64
}
