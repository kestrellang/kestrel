// test: diagnostics
// stdlib: false

module Test

struct Box[T] {
    let value: T
}
func main() {
    let b: Box[lang.i64] = Box[lang.i64](value: "hello"); // ERROR: type
}
