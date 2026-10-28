// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A static in a constrained extension (`where T: Equatable`) is available when
// the owner args satisfy it.

struct Box[T] { var value: T }

extend Box[T] where T: Equatable {
    static func same(a: T, b: T) -> Bool { a == b }
}

@main
func main() -> Int64 {
    if not Box[Int64].same(2, 2) { return 1 }
    if Box.same("a", "b") { return 2 }
    0
}
