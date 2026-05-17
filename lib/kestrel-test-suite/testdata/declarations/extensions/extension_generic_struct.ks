// test: diagnostics
// stdlib: false
module Test
struct Box[T] { var value: T }
extend Box[T] {
    func read() -> T { return self.value; }
}
func test() -> lang.i64 {
    let b = Box[lang.i64](value: 42);
    return b.read();
}
