// test: diagnostics
// stdlib: false

module Test

protocol Converter[From] {
    type Output
    func convert(input: From) -> Output
}
