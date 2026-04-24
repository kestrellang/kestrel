// test: diagnostics
// stdlib: false
module Test
protocol Hashable { }

enum Set[T] where T: Hashable {
    case Empty
    case NonEmpty(value: T)
}
