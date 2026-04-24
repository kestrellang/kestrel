// test: execution
// stdlib: true

module Test

protocol Valuable {
    func value() -> std.num.Int64
}

struct Token: Valuable {
    let v: std.num.Int64

    func value() -> std.num.Int64 {
        self.v
    }
}

func get_value[T](x: T) -> std.num.Int64 where T: Valuable {
    x.value()
}

func double_value[T](x: T) -> std.num.Int64 where T: Valuable {
    get_value[T](x) * 2
}

func main() -> lang.i64 {
    let t = Token(v: 21);
    if double_value[Token](t) != 42 { return 1 }
    0
}
