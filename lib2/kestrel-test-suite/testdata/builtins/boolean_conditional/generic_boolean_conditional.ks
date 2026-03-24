// test: diagnostics
// stdlib: false

module Test
struct Box[T] {
    var hasValue: lang.i1
    var value: T
}
extend Box[T]: Prelude.BooleanConditional {
    func asBool() -> lang.i1 {
        self.hasValue
    }
}
