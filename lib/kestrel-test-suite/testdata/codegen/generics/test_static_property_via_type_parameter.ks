// test: execution
// stdlib: true

module Test

protocol HasDefault {
    static var defaultValue: std.num.Int64 { get }
}

struct Config: HasDefault {
    static var defaultValue: std.num.Int64 { 100 }
}

func getDefault[T]() -> std.num.Int64 where T: HasDefault {
    T.defaultValue
}

func main() -> std.num.Int64 {
    if getDefault[Config]() != 100 { return 1 }
    0
}
