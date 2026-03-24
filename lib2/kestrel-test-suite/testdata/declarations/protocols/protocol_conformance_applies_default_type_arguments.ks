// test: diagnostics
// stdlib: false

module Test

protocol Multipliable[Rhs = Self] {
    func multiply(other: Rhs) -> Self
}
struct Box: Multipliable {
    init() { }
    func multiply(other: Box) -> Box { Box() }
}
