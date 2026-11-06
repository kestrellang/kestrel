// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Overloaded static methods on a generic type (an `OverloadSet` callee):
// the chosen overload still gets the owner's args.

struct Box[T] {
    static func pick(v: T) -> Int64 { 1 }
    static func pick(v: T, w: T) -> Int64 { 2 }
}

@main
func main() -> Int64 {
    if Box[Int64].pick(1) != 1 { return 1 }
    if Box.pick("a", "b") != 2 { return 2 }
    0
}
