// test: diagnostics
// stdlib: false
module Test

protocol Incrementable {
    mutating func increment()
}
struct Counter: Incrementable {
    mutating func increment() { }
}
