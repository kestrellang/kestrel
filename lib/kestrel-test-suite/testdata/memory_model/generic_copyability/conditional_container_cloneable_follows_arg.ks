// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {}

// A conditional container: move-only by default, Copyable when its arg is.
enum Box[T]: not Copyable {
    case Of(T)
}
extend Box[T]: Copyable where T: Copyable { }

// Cloneable element (explicit Cloneable conformance).
struct Res: Cloneable {
    var x: lang.i64
}

func needsCloneable[U](x: U) where U: Cloneable { }

// Box[Res]: Cloneable holds — all gating args Copyable AND Res is Cloneable.
// The conditional `extend` only declares Copyable, but the solver derives the
// Cloneable conformance from the gating arg.
func ok(b: Box[Res]) {
    needsCloneable(b);
}

// Box[lang.i64]: lang.i64 is Copyable but NOT Cloneable, so Box[i64] is not
// Cloneable.
func bad(b: Box[lang.i64]) {
    needsCloneable(b); // ERROR: !: Cloneable
}
