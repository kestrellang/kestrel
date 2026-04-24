// test: diagnostics
// stdlib: false

module Test

protocol Writable {
    var data: lang.i64 { get set }
}

struct Buffer: Writable {
    var data: lang.i64
}
