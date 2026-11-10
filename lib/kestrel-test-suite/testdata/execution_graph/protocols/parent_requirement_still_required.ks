// test: diagnostics
// stdlib: false

// The parent protocol is implied, but its requirements are not optional:
// leaving out `a()` is reported against the parent.
module Test

protocol A {
    func a()
}

protocol B: A {
    func b()
}

struct S: B { // ERROR: type 'S' does not implement method 'a' from protocol 'A'
    func b() { }
}
