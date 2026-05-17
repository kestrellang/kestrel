// test: diagnostics
// stdlib: false

// Regression: inside `extend Proto[T]`, calling a protocol method whose return
// type uses `T` incorrectly substituted `T` → `Self`. The solver mapped protocol
// type params to the receiver (SelfType) instead of the extension's corresponding
// type param TyVar.

module Test

struct Wrapper[T] { var inner: T }

protocol Readable[T] {
    func read() -> Wrapper[T]
}

extend Readable[T] {
    func readInner() -> T {
        let w: Wrapper[T] = self.read();
        w.inner
    }

    func readTwice() -> Wrapper[T] {
        return self.read()
    }
}

struct Source { }

extend Source: Readable[lang.i64] {
    func read() -> Wrapper[lang.i64] { return Wrapper(inner: 42) }
}
