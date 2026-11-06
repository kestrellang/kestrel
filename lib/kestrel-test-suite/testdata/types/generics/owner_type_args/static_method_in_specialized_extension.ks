// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A static in `extend Box[Int64]` (no params of its own) is reachable as
// `Box[Int64].seven()`; the owner arg must agree with the extension target.

struct Box[T] { var value: T }

extend Box[Int64] {
    static func seven() -> Box[Int64] { Box(value: 7) }
}

@main
func main() -> Int64 {
    if Box[Int64].seven().value != 7 { return 1 }
    0
}
