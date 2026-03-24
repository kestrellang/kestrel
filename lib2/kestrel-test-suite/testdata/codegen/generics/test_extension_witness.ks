// test: execution
// stdlib: true

module Test

protocol Doubler {
    func double() -> std.num.Int64
}

struct Num {
    let value: std.num.Int64
}

extend Num: Doubler {
    func double() -> std.num.Int64 {
        self.value * 2
    }
}

func do_double[T](x: T) -> std.num.Int64 where T: Doubler {
    x.double()
}

func main() -> lang.i64 {
    let n = Num(value: 21);
    if do_double[Num](n) != 42 { return 1 }
    0
}
