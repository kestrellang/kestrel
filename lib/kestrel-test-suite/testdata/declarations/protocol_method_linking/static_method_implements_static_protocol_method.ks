// test: diagnostics
// stdlib: false
module Test

protocol Factory {
    static func create() -> lang.i64
}
struct Item: Factory {
    static func create() -> lang.i64 { 0 }
}
