// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A static method declared in `extend Box[T]` sees the extension's `T`.

struct Box[T] { var value: T }

extend Box[T] {
    static func wrap(v: T) -> Box[T] { Box(value: v) }
    static func size(v: T) -> Int64 { 9 }
}

@main
func main() -> Int64 {
    if Box[Int64].wrap(4).value != 4 { return 1 }
    if Box.wrap(5).value != 5 { return 2 }
    if Box.size("s") != 9 { return 3 }
    0
}
