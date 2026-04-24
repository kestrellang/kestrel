// test: execution
// stdlib: true

module Test

protocol Factory {
    static func create() -> Self
}

struct Widget: Factory {
    let value: std.num.Int64
    static func create() -> Self {
        Widget(value: 42)
    }
}

func make[T]() -> T where T: Factory {
    T.create()
}

func main() -> std.num.Int64 {
    let w: Widget = make[Widget]();
    if w.value != 42 { return 1 }
    0
}
