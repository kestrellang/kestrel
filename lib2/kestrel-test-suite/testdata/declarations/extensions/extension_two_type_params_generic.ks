// test: diagnostics
// stdlib: false
module Test

struct Pair[T, U] { var first: T; var second: U }
extend Pair[T, U] {
    func getFirst() -> T { return self.first; }
    func getSecond() -> U { return self.second; }
}
func test() -> lang.i64 {
    let p = Pair[lang.str, lang.i64](first: "hello", second: 42);
    return p.getSecond();
}
