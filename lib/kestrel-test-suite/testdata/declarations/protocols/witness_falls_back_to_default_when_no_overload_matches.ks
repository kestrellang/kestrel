// test: diagnostics
// stdlib: true
//
// Overloads are keyed by labels. Neither `combine(other: Float64)` nor
// `combine(other: String)` witnesses `Combine` (the protocol-extension
// default does), so the two are a duplicate `combine(_:)` signature.

module Test

protocol Combine {
    func combine(other: Self) -> Int64
}
extend Combine {
    func combine(other: Self) -> Int64 { 7 }
}

struct V: Combine {
    var x: Int64
    func combine(other: Float64) -> Int64 { 1 }
    func combine(other: String) -> Int64 { 2 } // ERROR: duplicate function signature
}
