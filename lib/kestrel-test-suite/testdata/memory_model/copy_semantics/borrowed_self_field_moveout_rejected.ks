// test: diagnostics
// stdlib: false

// The legal/illegal boundary for field move-out (#145/#152): extracting a
// non-Copyable field is allowed out of a *consuming* receiver (it consumes the
// whole value — see consuming_self_field_moveout_execution.ks) but NOT out of a
// borrowed one. Returning a non-Copyable field out of a borrowed parameter moves
// it out of a borrow → E503. This pins that the consuming-self destructure fix
// did not also start permitting the borrowed case.
module Test

struct Res: not Copyable {
    var id: lang.i64
}

struct K {
    var inner: Res
}

func leak(k: K) -> Res {
    return k.inner; // ERROR(E503)
}
