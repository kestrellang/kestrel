// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A static computed property on a generic type, read through `Box[Int64]`.

struct Box[T] {
    static var tag: Int64 { 42 }
}

@main
func main() -> Int64 {
    if Box[Int64].tag != 42 { return 1 }
    0
}
