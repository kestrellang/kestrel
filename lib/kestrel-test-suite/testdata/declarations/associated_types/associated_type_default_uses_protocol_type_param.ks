// test: diagnostics
// stdlib: false
module Test

struct Pair[A, B] { }
protocol Mapping[K, V] {
    type Entry = Pair[K, V];
}
