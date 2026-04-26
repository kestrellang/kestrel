// test: diagnostics
// stdlib: false

module Test

protocol Mutable {
    var value: lang.i64 { get set }
}

struct Immutable: Mutable { // ERROR: setter
    var value: lang.i64 {
        get { 0 }
    }
}
