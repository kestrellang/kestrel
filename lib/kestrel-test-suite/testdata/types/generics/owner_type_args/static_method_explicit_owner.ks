// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Box[Int64].make(3)`: the receiver's `[Int64]` binds `Box`'s `T`.

struct Box[T] {
    var value: T
    static func make(v: T) -> Box[T] { Box(value: v) }
}

@main
func main() -> Int64 {
    let b = Box[Int64].make(3);
    if b.value != 3 { return 1 }
    0
}
