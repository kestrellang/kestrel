// test: diagnostics
// stdlib: false

module Test

protocol Base {
    static func create() -> Self
}
protocol Child: Base {
    func extra() -> lang.i64
}
func make[T]() -> T where T: Child {
    return T.create()
}
