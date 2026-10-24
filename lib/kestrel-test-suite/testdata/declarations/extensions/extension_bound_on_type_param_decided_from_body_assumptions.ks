// test: diagnostics
// stdlib: true

// bidi §7.2: a where-clause goal on a type parameter is decided against the
// asking body's assumptions Γ — the param's own declaration plus the where
// clauses of the body and its enclosing items. `noBound` has no `U: Show`
// anywhere, so `Box[U].show()` (only in `extend Box[T]: Show where T: Show`)
// is rejected where it is written. Before, the kernel answered "unknown" and
// the solver permitted it; the program then failed after specialization
// ("post-mono verification failed"), pointing inside the extension, and only
// when some caller passed a non-Show type. `methodBound` states the bound on
// the method, not on `U`'s declaration, and must stay accepted.
module Test

protocol Show {
    func show() -> Int64
}

struct Box[T] {
    var v: T;
}

extend Box[T]: Show where T: Show {
    func show() -> Int64 { self.v.show() }
}

func noBound[U](b: Box[U]) -> Int64 {
    b.show() // ERROR: no member 'show' on type 'Box[U]'
}

func withBound[U](b: Box[U]) -> Int64 where U: Show {
    b.show()
}

struct Wrapper[U] {
    var b: Box[U];
}

extend Wrapper[U] {
    func methodBound() -> Int64 where U: Show {
        self.b.show()
    }
}
