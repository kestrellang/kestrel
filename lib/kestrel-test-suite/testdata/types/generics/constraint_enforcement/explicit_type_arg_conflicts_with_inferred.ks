// test: diagnostics
// stdlib: false

module Test

func identity[T](x: T) -> T { return x }
func main() {
    let y: lang.i64 = identity[lang.str](42); // ERROR: type
}
