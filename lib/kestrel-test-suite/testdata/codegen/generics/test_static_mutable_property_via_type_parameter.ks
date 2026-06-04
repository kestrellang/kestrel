// test: execution
// stdlib: true

module Test

protocol Counter {
    static var count: std.numeric.Int64 { get set }
}

struct MyCounter: Counter {
    static var _count: std.numeric.Int64 = 0
    static var count: std.numeric.Int64 {
        get { MyCounter._count }
        set { MyCounter._count = newValue }
    }
}

func increment[T]() where T: Counter {
    T.count = T.count + 1
}

@main
func main() -> lang.i64 {
    increment[MyCounter]();
    if MyCounter.count != 1 { return 1 }
    0
}
