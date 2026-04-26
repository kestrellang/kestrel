// test: diagnostics
// stdlib: false

module Main
struct S {
    private func privateMethod() { }
}

func test() {
    let s = S();
    s.privateMethod() // ERROR: is private and not accessible from this scope
}
