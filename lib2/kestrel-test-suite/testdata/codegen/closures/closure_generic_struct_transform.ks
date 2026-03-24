// test: execution
// stdlib: true

module Test

struct Transform[T, U] {
    let transform: (T) -> U
}

func main() -> lang.i64 {
    let t = Transform[std.num.Int64, std.num.Int64](transform: { (x) in x * 2 });
    if (t.transform)(21) != 42 { return 1 }
    0
}
