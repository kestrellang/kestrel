// test: diagnostics
// stdlib: false

module Main

struct A {
    let b: B // ERROR: circular struct containment
}

struct B {
    let a: A
}
