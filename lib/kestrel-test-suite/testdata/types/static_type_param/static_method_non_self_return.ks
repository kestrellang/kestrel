// test: diagnostics
// stdlib: false

module Test

protocol Describable {
    static func typeName() -> lang.str
}
func getName[T]() -> lang.str where T: Describable {
    return T.typeName()
}
