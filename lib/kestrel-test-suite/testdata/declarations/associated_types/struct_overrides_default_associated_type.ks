// test: diagnostics
// stdlib: false
module Test

protocol Parser {
    type Output = lang.str;
    func parse() -> Output
}
struct IntParser: Parser {
    type Output = lang.i64;
    func parse() -> lang.i64 { 0 }
}
