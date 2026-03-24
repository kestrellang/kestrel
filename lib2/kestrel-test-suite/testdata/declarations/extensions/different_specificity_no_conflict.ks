// test: diagnostics
// stdlib: false
module Test
struct Box[T] { var value: T }
extend Box[T] {
    func describe() -> lang.str { return "generic"; }
}
extend Box[lang.i64] {
    func describe() -> lang.str { return "lang.i64"; }
}
