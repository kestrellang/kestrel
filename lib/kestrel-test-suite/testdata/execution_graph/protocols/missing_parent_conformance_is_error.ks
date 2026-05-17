// test: diagnostics
// stdlib: false

module Test

protocol A {
    func a()
}

protocol B: A {
    func b()
}

struct S: B { // ERROR: conforms to 'B' but not its parent protocol 'A'
    func a() { }
    func b() { }
}
