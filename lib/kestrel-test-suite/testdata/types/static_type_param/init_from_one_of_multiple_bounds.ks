// test: diagnostics
// stdlib: false

module Test

protocol Creatable {
    init()
}
protocol Describable {
    func describe() -> lang.str
}
func make[T]() -> T where T: Creatable, T: Describable {
    return T()
}
