// test: diagnostics
// stdlib: false

// A NotCopyable element cannot ride the get→op→set writeback (the copy
// through `get` is the whole mechanism) — the copy guard fires with the
// "add a `mutating ref` accessor" hint. (The setter deliberately ignores
// `newValue`: a setter that STORES a borrowed NotCopyable newValue is
// itself an E503 at its own body — settable NotCopyable elements need
// the mutating-ref world either way.)
module Test

struct Res: not Copyable {
    var v: lang.i64
    // (avoids `+`: lang.i64 has no Addable without the stdlib)
    mutating func bump() { self.v = 1; }
}

struct Holder {
    var stored: Res
    subscript(at index: lang.i64) -> Res {
        get { Res(v: self.stored.v) }
        set { }
    }
}

func use() {
    var h = Holder(stored: Res(v: 5));
    h(at: 0).bump(); // ERROR(E503)
}
