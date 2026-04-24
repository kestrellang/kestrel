// test: execution
// stdlib: true

// Regression: init calls inside generic function/extension bodies used to get
// struct type args twice. MIR lowering's `lower_call` fell back to the Def's
// explicit type args (which are the struct's type args) AND then
// `emit_call_maybe_init` prepended those same struct type args via
// `prepend_receiver_type_args`. Result: a 1-type-param init was handed 2
// type args — one of them often `Error` from an unresolved associated-type
// projection. Monomorphizer skipped the instantiation (Error triggers
// `has_type_param`), and codegen failed with "call to undeclared function:
// Array.init(...)".
//
// This test uses the same shape as `Array.flatten()` — `Array[T.Item]()` in a
// generic extension — but only as a return value (no locals with T.Item
// types), so it isolates the init double-prepend bug from the flatten-style
// codegen issue with associated-type locals.

module Test

extend std.collections.Array[T] where T: std.iter.Iterable {
    public func emptyBag() -> std.collections.Array[T.Item] {
        std.collections.Array[T.Item]()
    }
}

func main() -> lang.i64 {
    var outer = std.collections.Array[std.collections.Array[std.num.Int64]]();
    // T resolves to Array[Int64], so T.Item is Int64 and emptyBag() returns
    // an empty Array[Int64]. Before the fix, Array.init inside emptyBag was
    // mangled with two type args — one Int64, one Error — and the
    // monomorphizer skipped it, so the link step failed.
    let bag = outer.emptyBag();
    if bag.count != 0 { return 1 }
    0
}
