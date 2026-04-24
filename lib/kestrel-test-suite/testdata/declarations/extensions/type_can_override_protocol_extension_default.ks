// test: diagnostics
// stdlib: false
module Test

protocol Describable {
    func describe()
}
extend Describable {
    func describe() { }
}
struct Item: Describable {
    func describe() { }
}
func test() {
    let i = Item();
    i.describe();
}
