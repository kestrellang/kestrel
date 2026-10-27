// test: diagnostics

// Moving a non-Copyable field out of a value whose type has a `deinit` would
// take the value apart without running the deinit. It used to compile and
// silently skip the deinit. A type without a deinit, and the type's own
// deinit, may still move fields out.

module Test

struct Inner: not Copyable { var v: Int64 }

struct Guarded: not Copyable {
    var inner: Inner
    deinit { }
}

struct Plain: not Copyable {
    var inner: Inner
}

func fromGuarded(consuming g: Guarded) -> Inner {
    g.inner // ERROR(E511): cannot move a field out of 'Guarded', which has a deinit
}

func fromPlain(consuming p: Plain) -> Inner {
    p.inner
}
