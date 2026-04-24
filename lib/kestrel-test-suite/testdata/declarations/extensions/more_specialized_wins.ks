// test: diagnostics
// stdlib: false
module Test

struct Pair[T, U] { var first: T; var second: U }
extend Pair[T, U] {
    func describe() -> lang.str { return "generic pair"; }
}
extend Pair[T, lang.i64] {
    func describe() -> lang.str { return "half specialized"; }
}
extend Pair[lang.i64, lang.i64] {
    func describe() -> lang.str { return "fully specialized"; }
}
func test1() -> lang.str {
    let p = Pair[lang.str, lang.str](first: "a", second: "b");
    return p.describe();
}
func test2() -> lang.str {
    let p = Pair[lang.str, lang.i64](first: "a", second: 1);
    return p.describe();
}
func test3() -> lang.str {
    let p = Pair[lang.i64, lang.i64](first: 1, second: 2);
    return p.describe();
}
