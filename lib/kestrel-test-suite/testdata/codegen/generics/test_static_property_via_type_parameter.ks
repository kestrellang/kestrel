// test: execution
// stdlib: true

module Test

protocol HasDefault {
    static var defaultValue: std.numeric.Int64 { get }
}

struct Config: HasDefault {
    static var defaultValue: std.numeric.Int64 { 100 }
}

func getDefault[T]() -> std.numeric.Int64 where T: HasDefault {
    T.defaultValue
}

@main
func main() -> lang.i64 {
    if getDefault[Config]() != 100 { return 1 }
    0
}
