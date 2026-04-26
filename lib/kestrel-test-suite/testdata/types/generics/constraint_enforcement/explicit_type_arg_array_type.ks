// test: diagnostics
// stdlib: true

module Test

func identity[T](x: T) -> T { return x }
func main() {
    let arr: [lang.i64] = identity[[lang.i64]]([1, 2, 3]);
}
