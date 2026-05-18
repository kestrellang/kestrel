// test: diagnostics
// stdlib: false

// `some` with a non-protocol bound (struct). Currently no error is
// emitted — the struct bound is silently ignored since it doesn't
// resolve to TyKind::Protocol in the opaque return handling.
// TODO: implement E466 "some requires a protocol bound" diagnostic.

module Test

struct Concrete {
    public init() {}
}

func bad() -> some Concrete {
    Concrete()
}
