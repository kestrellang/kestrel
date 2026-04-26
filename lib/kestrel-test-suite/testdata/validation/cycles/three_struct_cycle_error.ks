// test: diagnostics
// stdlib: false

module Main

struct A {
    let b: B // ERROR: circular struct containment
}

struct B {
    let c: C
}

struct C {
    let a: A
}
