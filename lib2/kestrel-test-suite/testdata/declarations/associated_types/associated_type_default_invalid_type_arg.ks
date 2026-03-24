// test: diagnostics
// stdlib: false
module Test

struct Array[T] { }
protocol Collection[T] {
    type Storage = Array[Unknown]; // ERROR: cannot find type
}
