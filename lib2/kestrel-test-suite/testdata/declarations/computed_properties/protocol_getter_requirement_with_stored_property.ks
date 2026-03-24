// test: diagnostics
// stdlib: false

module Test

protocol Identifiable {
    var id: lang.i64 { get }
}

struct Entity: Identifiable {
    var id: lang.i64
}
