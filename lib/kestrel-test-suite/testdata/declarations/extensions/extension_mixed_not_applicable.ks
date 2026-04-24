// test: diagnostics
// stdlib: false
module Test

struct Pair[T, U] { var first: T; var second: U }
extend Pair[T, lang.i64] {
    func getSecond() -> lang.i64 { return self.second; }
}
func test() -> lang.str {
    let p = Pair[lang.str, lang.str](first: "hello", second: "world");
    return p.getSecond(); // ERROR: getSecond
}
