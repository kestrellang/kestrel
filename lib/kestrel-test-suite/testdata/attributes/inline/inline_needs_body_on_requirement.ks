// test: diagnostics
// stdlib: false

module Test

protocol Shape {
    @inline // ERROR: '@inline' needs a body; a protocol requirement has none
    func area() -> lang.i64
}
