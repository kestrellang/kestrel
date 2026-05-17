// test: diagnostics
// stdlib: false

// Regression: inside `extend Proto[T]`, a subscript[I] set block incorrectly
// resolved T as I. The solver synthesized receiver type args from the setter's
// direct parent (the subscript, which has TypeParams [I]) instead of traversing
// to the enclosing extension. This caused the protocol's T to be mapped to the
// subscript's I in substitution, producing "expected I got T" mismatches.

module Test

protocol Idx {
    func raw() -> lang.i64
}

protocol Store[T] {
    func readAt(index idx: lang.i64) -> T
    func writeAt(index idx: lang.i64, value val: T)
}

extend Store[T] {
    subscript[I](at index: I) -> T where I: Idx {
        get { self.readAt(index: index.raw()) }
        set { self.writeAt(index: index.raw(), value: newValue) }
    }
}

struct IntIdx { var v: lang.i64 }

extend IntIdx: Idx {
    func raw() -> lang.i64 { self.v }
}

struct Shelf[T] { var slot: T }

extend Shelf[T]: Store[T] {
    func readAt(index idx: lang.i64) -> T { self.slot }
    func writeAt(index idx: lang.i64, value val: T) { }
}
