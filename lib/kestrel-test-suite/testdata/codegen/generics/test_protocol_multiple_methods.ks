// test: execution
// stdlib: true

module Test

protocol Math {
    func add(other: Self) -> Self
    func value() -> std.numeric.Int64
}

struct Num: Math {
    let n: std.numeric.Int64

    func add(other: Num) -> Num {
        Num(n: self.n + other.n)
    }

    func value() -> std.numeric.Int64 {
        self.n
    }
}

func sum_and_get[T](a: T, b: T) -> std.numeric.Int64 where T: Math {
    let result = a.add(b);
    result.value()
}

@main
func main() -> lang.i64 {
    let a = Num(n: 20);
    let b = Num(n: 22);
    if sum_and_get[Num](a, b) != 42 { return 1 }
    0
}
