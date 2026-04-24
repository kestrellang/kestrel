// test: diagnostics
// stdlib: false

module Test

struct Box[T] { var value: T }

protocol Boxable {
    func box() -> Box[Self]
}
