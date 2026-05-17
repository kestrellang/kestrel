// test: diagnostics
// stdlib: false
module Test
protocol Hashable {
    func hash() -> lang.i64
}
struct Point: Hashable {
    func hash() -> lang.str { } // ERROR: method 'hash' has wrong return type
}
