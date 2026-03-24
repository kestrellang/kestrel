// test: diagnostics
// stdlib: false

module Test

func identity[T](x: T) -> T { return x }
func main() {
    let x: lang.i64 = lang.i64_add(identity[lang.i64](1), identity[lang.i64](2));
}
