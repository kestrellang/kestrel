// test: execution
// stdlib: true

module Test

struct Transform[T, U] {
    let transform: (T) -> U
}

@main
func main() -> lang.i64 {
    let t = Transform[std.numeric.Int64, std.numeric.Int64](transform: { (x) in x * 2 });
    if (t.transform)(21) != 42 { return 1 }
    0
}
