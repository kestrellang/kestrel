// test: execution
// stdlib: false

// Regression: a method on a struct that takes another struct as a parameter
// and reads one of that parameter's fields returns the wrong value.
//
// Root cause: lower_call_args classified the init call's primitive literal
// argument by reference because resolve_expr_type returned MirTy::Error
// (inference left the literal TyVar unresolved). The init body stored that
// reference directly into the field, so the constructed struct held a stack
// pointer instead of the scalar; the subsequent field read through the
// method's struct parameter returned that pointer-low-byte instead of the
// original value.

module Test

struct Box {
    var v0: lang.i64

    public init(v0 v0: lang.i64) { self.v0 = v0 }
}

struct Idx {
    var value: lang.i64

    public init(value value: lang.i64) { self.value = value }

    public func access(box box: Box) -> lang.i64 {
        box.v0
    }
}

@main
func main() -> lang.i64 {
    let b = Box(v0: 30);
    let i = Idx(value: 2);
    let e = i.access(box: b);
    if lang.i64_ne(e, 30) { return 1 }
    0
}
