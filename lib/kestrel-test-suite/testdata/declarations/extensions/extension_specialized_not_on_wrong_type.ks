// test: diagnostics
// stdlib: false
module Test
struct Box[T] { var value: T }
extend Box[lang.i64] {
    func doubled() -> lang.i64 { return lang.i64_mul(self.value, 2); }
}
func test() -> lang.i64 {
    let b = Box[lang.str](value: "hello");
    return b.doubled(); // ERROR: doubled
}
