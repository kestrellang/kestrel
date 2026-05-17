// test: diagnostics
// stdlib: false
module Test

protocol Factory {
    type Product;
    func make() -> Product
}
struct Maker { }
extend Maker: Factory { // ERROR: does not provide associated type 'Product'
    func make() -> lang.i64 { return 1; }
}
