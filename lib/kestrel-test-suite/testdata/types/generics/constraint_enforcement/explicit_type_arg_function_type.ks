// test: diagnostics
// stdlib: false

module Test

func identity[T](x: T) -> T { return x }
func myFunc(x: lang.i64) -> lang.i64 { return x }
func main() {
    let f: (lang.i64) -> lang.i64 = identity[(lang.i64) -> lang.i64](myFunc);
}
