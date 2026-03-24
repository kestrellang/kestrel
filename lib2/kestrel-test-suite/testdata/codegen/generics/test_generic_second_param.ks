// test: execution
// stdlib: true

module Test

func second[A, B](a: A, b: B) -> B {
    b
}

func main() -> lang.i64 {
    if second[std.core.Bool, std.num.Int64](true, 42) != 42 { return 1 }
    0
}
