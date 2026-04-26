// test: execution
// stdlib: true

// Regression: explicit type args on a method call (e.g., `f.make[Int]()`) used
// to be dropped because the `Member` constraint had no field for them.
// The solver created a fresh TyVar for the method's own type param and never
// equated it with the explicit arg. When no downstream coercion constrained
// the TyVar, it stayed unresolved → MirTy::Error leaked into monomorphization
// → codegen/link failed with "call to undeclared function".
//
// This test calls a generic method and *discards the result*, so the fresh
// TyVar for `U` has no downstream coercion to rescue it. Without the fix,
// monomorphization sees `[Error]` as method_type_args and skips the
// instantiation; the link step then fails.

module Test

protocol Factory {
    func make[U](seed: lang.i64) -> U
}

struct IntMaker { }
extend IntMaker: Factory {
    func make[U](seed: lang.i64) -> U {
        lang.panic("IntMaker.make never called in this test")
    }
}

func main() -> lang.i64 {
    if false {
        let m = IntMaker();
        // Return value discarded — `U` is only constrained by the explicit
        // type arg. If that path is broken, U stays Error and the call fails
        // to monomorphize/link.
        m.make[std.num.Int64](0);
    }
    0
}
