// test: execution
// stdlib: true

module Test

struct Box[T] {
    let value: T
}

func main() -> lang.i64 {
    let b = Box[std.num.Int64](value: 42);
    if b.value != 42 { return 1 }
    0
}
