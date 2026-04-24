// test: diagnostics
// stdlib: false
module Test

protocol Factory {
    static func create() -> Self
}
struct Item: Factory {
    func create() -> Item { } // ERROR: receiver
}
