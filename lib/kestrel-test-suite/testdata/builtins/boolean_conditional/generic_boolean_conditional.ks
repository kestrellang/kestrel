// test: diagnostics
// stdlib: true

module Test
struct Box[T] {
    var hasValue: lang.i1
    var value: T
}
extend Box[T]: BooleanConditional {
    func boolValue() -> lang.i1 {
        self.hasValue
    }
}
