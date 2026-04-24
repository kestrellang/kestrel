// test: execution
// stdlib: true

module Test

protocol Container {
    func read() -> std.num.Int64
}

struct Wrapper[T]: Container {
    let value: std.num.Int64

    func read() -> std.num.Int64 {
        self.value
    }
}

func extract[C](c: C) -> std.num.Int64 where C: Container {
    c.read()
}

func main() -> lang.i64 {
    let w = Wrapper[std.core.Bool](value: 42);
    if extract[Wrapper[std.core.Bool]](w) != 42 { return 1 }
    0
}
