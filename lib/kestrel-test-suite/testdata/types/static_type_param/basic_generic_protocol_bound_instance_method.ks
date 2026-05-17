// test: diagnostics
// stdlib: false

module Test

protocol Converter[Target] {
    func convert() -> Target
}
func useConverter[T](val: T) -> lang.i64 where T: Converter[lang.i64] {
    val.convert()
}
