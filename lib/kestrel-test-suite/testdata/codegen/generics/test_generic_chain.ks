// test: execution
// stdlib: true

module Test

func step1[T](x: T) -> T { x }
func step2[T](x: T) -> T { step1[T](x) }
func step3[T](x: T) -> T { step2[T](x) }

func main() -> lang.i64 {
    if step3[std.numeric.Int64](42) != 42 { return 1 }
    0
}
