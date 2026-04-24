// test: diagnostics
// stdlib: false

module Test

protocol BiConverter[From, To] {
    func convert(input: From) -> To
}
func transform[T](c: T, input: lang.str) -> lang.i64 where T: BiConverter[lang.str, lang.i64] {
    c.convert(input)
}
