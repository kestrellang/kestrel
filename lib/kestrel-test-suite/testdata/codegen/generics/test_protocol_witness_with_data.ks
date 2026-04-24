// test: execution
// stdlib: true

module Test

protocol Valuable {
    func value() -> std.num.Int64
}

struct Box: Valuable {
    let inner: std.num.Int64

    func value() -> std.num.Int64 {
        self.inner
    }
}

func get_value[T](x: T) -> std.num.Int64 where T: Valuable {
    x.value()
}

func main() -> lang.i64 {
    let b = Box(inner: 42);
    if get_value[Box](b) != 42 { return 1 }
    0
}
