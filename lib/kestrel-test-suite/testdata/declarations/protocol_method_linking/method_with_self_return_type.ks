// test: diagnostics
// stdlib: false
module Test

protocol Cloneable {
    func clone() -> Self
}
struct Point: Cloneable {
    func clone() -> Point { Point() }
}
