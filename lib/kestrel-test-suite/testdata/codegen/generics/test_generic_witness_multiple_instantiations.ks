// test: execution
// stdlib: true

module Test

protocol Container {
    func read() -> std.num.Int64
}

struct Box[T]: Container {
    let value: std.num.Int64

    func read() -> std.num.Int64 {
        self.value
    }
}

func extract[C](c: C) -> std.num.Int64 where C: Container {
    c.read()
}

func main() -> lang.i64 {
    let b1 = Box[std.num.Int64](value: 20);
    let b2 = Box[std.core.Bool](value: 22);
    if extract[Box[std.num.Int64]](b1) + extract[Box[std.core.Bool]](b2) != 42 { return 1 }
    0
}
