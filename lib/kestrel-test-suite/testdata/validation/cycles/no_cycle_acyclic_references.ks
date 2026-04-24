// test: diagnostics
// stdlib: false

module Main

struct A {
    let value: lang.i64
}

struct B {
    let a: A
}

struct C {
    let b: B
}
