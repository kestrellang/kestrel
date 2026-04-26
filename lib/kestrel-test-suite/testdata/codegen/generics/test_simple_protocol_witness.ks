// test: execution
// stdlib: true

module Test

protocol Valuable {
    func value() -> std.num.Int64
}

struct Token: Valuable {
    func value() -> std.num.Int64 {
        42
    }
}

func get_value[T](x: T) -> std.num.Int64 where T: Valuable {
    x.value()
}

func main() -> lang.i64 {
    let t = Token();
    if get_value[Token](t) != 42 { return 1 }
    0
}
