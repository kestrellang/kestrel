// test: diagnostics
// stdlib: false

module Test

func identity[T](x: T) -> T { return x }
func main() {
    let f = identity[lang.str];
    let x: lang.i64 = f("hello"); // ERROR: type
}
