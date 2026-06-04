// test: execution
// stdlib: true

module Test

protocol Valuable {
    func value() -> std.numeric.Int64
}

struct Token: Valuable {
    func value() -> std.numeric.Int64 {
        42
    }
}

func get_value[T](x: T) -> std.numeric.Int64 where T: Valuable {
    x.value()
}

@main
func main() -> lang.i64 {
    let t = Token();
    if get_value[Token](t) != 42 { return 1 }
    0
}
