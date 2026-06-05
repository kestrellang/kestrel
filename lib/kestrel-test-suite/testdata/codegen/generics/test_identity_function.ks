// test: execution
// stdlib: true

module Test

func identity[T](x: T) -> T {
    x
}

@main
func main() -> lang.i64 {
    if identity[std.numeric.Int64](42) != 42 { return 1 }
    0
}
