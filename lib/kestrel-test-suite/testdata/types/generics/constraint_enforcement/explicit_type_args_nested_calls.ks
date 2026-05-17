// test: diagnostics
// stdlib: false

module Test

func wrap[T](x: T) -> T { return x }
func main() {
    let x: lang.i64 = wrap[lang.i64](wrap[lang.i64](42));
}
