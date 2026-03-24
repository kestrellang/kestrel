// test: diagnostics
// stdlib: false
module Test

struct Array[T] { }
protocol Collection[T] {
    type Storage = Array[T];
}
