// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    static func create() -> Self
}
protocol Describable {
    func describe() -> lang.str
}
func makeAndDescribe[T]() -> lang.str where T: Factory, T: Describable {
    let item: T = T.create();
    return item.describe()
}
