// test: execution
// stdlib: true

module Test

func first[A, B](a: A, b: B) -> A {
    a
}

@main
func main() -> lang.i64 {
    if first[std.numeric.Int64, std.core.Bool](42, true) != 42 { return 1 }
    0
}
