// test: diagnostics
// stdlib: false

module Test

func foo(x: lang.i64) -> lang.i64 { return x }
func main() {
    let y: lang.i64 = foo[lang.i64](42); // ERROR: does not accept type arguments
}
