// test: execution
// stdlib: true

module Test

struct Box[T] {
    let value: T
}

func unbox[T](b: Box[T]) -> T {
    b.value
}

func main() -> lang.i64 {
    let b = Box[std.num.Int64](value: 42);
    if unbox[std.num.Int64](b) != 42 { return 1 }
    0
}
