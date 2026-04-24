// test: diagnostics
// stdlib: false
module Test

struct Pair[T, U] { var first: T; var second: U }
extend Pair[T, lang.i64] {
    func getSecond() -> lang.i64 { return self.second; }
}
func test() -> lang.i64 {
    let p = Pair[lang.str, lang.i64](first: "hello", second: 42);
    return p.getSecond();
}
