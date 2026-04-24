// test: execution
// stdlib: true

module Test

func identity[T](x: T) -> T {
    x
}

func main() -> lang.i64 {
    let a = identity[std.num.Int64](40);
    let b = identity[std.core.Bool](true);
    let c = identity[std.num.Int64](2);
    if a + c != 42 { return 1 }
    0
}
