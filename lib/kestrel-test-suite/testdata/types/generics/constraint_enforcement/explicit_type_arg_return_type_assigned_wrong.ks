// test: diagnostics
// stdlib: false

module Test

struct Box[T] {
    let value: T
}
func wrap[T](x: T) -> Box[T] { return Box[T](value: x) }
func main() {
    let b: Box[lang.str] = wrap[lang.i64](42); // ERROR: type
}
