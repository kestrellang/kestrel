// test: diagnostics
// stdlib: false

module Main

struct Inner[T] {
    let value: T
}

struct Outer[T] {
    let inner: Inner[T]
}
