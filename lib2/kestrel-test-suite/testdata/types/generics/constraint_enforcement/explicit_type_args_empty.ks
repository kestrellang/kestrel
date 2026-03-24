// test: diagnostics
// stdlib: false

module Test

func identity[T](x: T) -> T { return x }
func main() {
    let y: lang.i64 = identity[](42); // ERROR: empty type argument list
}
