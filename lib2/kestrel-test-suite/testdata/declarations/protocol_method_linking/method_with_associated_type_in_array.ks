// test: diagnostics
// stdlib: true
module Test

protocol Collection {
    type Element;
    func getAll() -> [Element]
}
struct IntArray: Collection {
    type Element = lang.i64;
    func getAll() -> [lang.i64] { [] }
}
