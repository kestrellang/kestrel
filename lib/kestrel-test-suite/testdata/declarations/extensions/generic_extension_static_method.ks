// test: diagnostics
// stdlib: false
module Test
struct Box[T] { var value: T }
extend Box[lang.i64] {
    static func zero() -> Box[lang.i64] { return Box[lang.i64](value: 0); }
}
func test() -> Box[lang.i64] { return Box[lang.i64].zero(); }
