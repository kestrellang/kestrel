// test: diagnostics
// stdlib: false

module Main

struct A {
    let pair: (lang.i64, B)
}

struct B {
    let a: A // ERROR: circular struct containment
}
