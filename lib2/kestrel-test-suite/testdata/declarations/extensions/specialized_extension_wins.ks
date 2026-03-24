// test: diagnostics
// stdlib: false
module Test

struct Box[T] { var value: T }
extend Box[T] {
    func describe() -> lang.str { return "generic box"; }
}
extend Box[lang.i64] {
    func describe() -> lang.str { return "lang.i64 box"; }
}
func testGeneric() -> lang.str {
    let b = Box[lang.str](value: "hello");
    return b.describe();
}
func testSpecialized() -> lang.str {
    let b = Box[lang.i64](value: 42);
    return b.describe();
}
