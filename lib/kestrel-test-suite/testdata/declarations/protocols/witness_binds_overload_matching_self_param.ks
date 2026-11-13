// test: diagnostics
// stdlib: true
//
// Overloads are keyed by labels. `combine(other: Float64)` witnesses nothing
// (`V` conforms to `Combine` once, whose witness is `combine(other: V)`), so
// it duplicates the witness's `combine(_:)` signature. The legal form of this
// binding question — same-label witnesses of distinct instantiations — is
// `witness_binds_overload_matching_instantiation.ks`.

module Test

protocol Combine {
    func combine(other: Self) -> Int64
}

struct V: Combine {
    var n: Int64
    func combine(other: Float64) -> Int64 { 1 }
    func combine(other: V) -> Int64 { 2 } // ERROR: duplicate function signature
}
