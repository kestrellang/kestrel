// test: execution
// stdlib: true

module Test

struct Box[T] {
    let value: T
}

func unbox[T](b: Box[T]) -> T {
    b.value
}

@main
func main() -> lang.i64 {
    let b = Box[std.numeric.Int64](value: 42);
    if unbox[std.numeric.Int64](b) != 42 { return 1 }
    0
}
