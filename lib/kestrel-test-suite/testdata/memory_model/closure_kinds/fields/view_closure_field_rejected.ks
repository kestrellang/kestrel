// test: diagnostics
// stdlib: false

// A stored field or enum payload cannot hold a view-kind closure (normal or
// `mutating`), directly, inside a tuple, or through a plain type alias: a
// `mutating` method could store a closure viewing the caller's locals and the
// caller return the value (docs/design/closure-fields.md). Owning kinds are
// fine. E626.
module Test

type Handler = () -> lang.i64;

struct Plain {
    var f: () -> lang.i64 // ERROR(E626)
}

struct Mut {
    var f: mutating () -> () // ERROR(E626)
}

struct InTuple {
    var t: (() -> lang.i64, lang.i64) // ERROR(E626)
}

struct ViaAlias {
    var h: Handler // ERROR(E626)
}

struct Owning {
    var e: escaping () -> lang.i64;
    var c: consuming () -> lang.i64;
}

enum Event {
    case click(() -> ()) // ERROR(E626)
    case owned(escaping () -> ())
}
