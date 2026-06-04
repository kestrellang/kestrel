// test: execution
// stdlib: true

module Test

protocol Container {
    func read() -> std.numeric.Int64
}

struct Box[T]: Container {
    let value: std.numeric.Int64

    func read() -> std.numeric.Int64 {
        self.value
    }
}

func extract[C](c: C) -> std.numeric.Int64 where C: Container {
    c.read()
}

@main
func main() -> lang.i64 {
    let b1 = Box[std.numeric.Int64](value: 20);
    let b2 = Box[std.core.Bool](value: 22);
    if extract[Box[std.numeric.Int64]](b1) + extract[Box[std.core.Bool]](b2) != 42 { return 1 }
    0
}
