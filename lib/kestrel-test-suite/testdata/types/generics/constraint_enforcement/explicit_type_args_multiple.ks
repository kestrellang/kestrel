// test: diagnostics
// stdlib: false

module Test

func pair[A, B](a: A, b: B) -> A { return a }
func main() {
    let x: lang.i64 = pair[lang.i64, lang.str](1, "hello");
}
