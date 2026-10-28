// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Box[Int64](value: 3)`: the written args bind `Box`'s `T`.

struct Box[T] { var value: T }

@main
func main() -> Int64 {
    let b = Box[Int64](value: 3);
    if b.value != 3 { return 1 }
    0
}
