// test: execution
// stdlib: true

module Test

struct Box[T] {
    let value: T
}

struct Container[T] {
    let make: () -> Box[T]
}

func main() -> lang.i64 {
    let c = Container[std.num.Int64](make: { Box[std.num.Int64](value: 42) });
    let box = (c.make)();
    if box.value != 42 { return 1 }
    0
}
