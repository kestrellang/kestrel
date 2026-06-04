// test: execution
// stdlib: true

module Test

func identity[T](x: T) -> T {
    x
}

func wrap[T](x: T) -> T {
    identity[T](x)
}

@main
func main() -> lang.i64 {
    if wrap[std.numeric.Int64](42) != 42 { return 1 }
    0
}
