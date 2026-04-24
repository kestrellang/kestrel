// test: diagnostics
// stdlib: false
module Test

protocol Factory {
    type Product;
    func make() -> Product
}
struct Maker { }
extend Maker: Factory {
    type Product = lang.i64;
    func make() -> lang.i64 { return 1; }
}
func test() -> lang.i64 {
    let m = Maker();
    return m.make();
}
