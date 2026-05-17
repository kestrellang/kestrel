// test: diagnostics
// stdlib: false
module Test

protocol Container {
    type Item;
    func add(item: Item)
}
struct IntContainer: Container {
    type Item = lang.i64;
    func add(item: lang.i64) { }
}
func test() {
    let c = IntContainer();
    c.add(42);
}
