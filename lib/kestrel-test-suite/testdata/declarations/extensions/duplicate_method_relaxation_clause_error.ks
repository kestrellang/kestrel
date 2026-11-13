// test: diagnostics
// stdlib: false

// `where T: not Copyable` only relaxes T's implicit Copyable bound, so it
// doesn't narrow the extension: both extensions cover every `Box[T]`.

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Box[T] where T: not Copyable {
    var v: T
}

extend Box[T] {
    func name() -> lang.i64 { 1 }
}

extend Box[T] where T: not Copyable {
    func name() -> lang.i64 { 2 } // ERROR: duplicate
}
