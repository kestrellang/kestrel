// test: diagnostics
// stdlib: false

module Test

protocol Equatable { }
enum Set[T] where T: Equatable {
    case Empty
    case Elements(items: T)
}
