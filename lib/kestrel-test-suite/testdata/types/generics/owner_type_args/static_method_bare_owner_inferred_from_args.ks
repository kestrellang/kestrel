// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Box.make(3)` outside `Box`: bare `Box` is `Box[_]`; `T` comes from the argument.

struct Box[T] {
    var value: T
    static func make(v: T) -> Box[T] { Box(value: v) }
}

@main
func main() -> Int64 {
    let b = Box.make(3);
    if b.value != 3 { return 1 }
    0
}
