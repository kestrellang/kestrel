// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)

// Regression (#175/#176): immediately calling a closure produced by a
// call/subscript — `s.mk()()`, `fs(0)()`, `p(0)()` — must invoke the returned
// closure VALUE, not re-dispatch to the inner callee. MIR's emit_resolved_call
// fell back to the *inner* call's resolution when the outer callee was itself a
// call (re-calling the method with the closure as self → garbage (#175); or
// emitting a Slice / user `subscript` witness on the FuncThick → mono-verify
// fail / wrong-arity codegen (#176)).

struct S {
    func mk() -> () -> Int64 { { () in 7 } }
}

struct Provider {
    var f: () -> Int64
    subscript(i: Int64) -> () -> Int64 { get { self.f } }
}

@main
func main() -> lang.i64 {
    // #175: method-returned closure, called immediately.
    let s = S();
    if s.mk()() != 7 { return 1 }

    // #176a: array-subscript-returned closure, called immediately.
    let fs: [() -> Int64] = [{ () in 42 }];
    if fs(0)() != 42 { return 2 }

    // #176b: user-subscript-returned closure, called immediately.
    let p = Provider(f: { () in 9 });
    if p(0)() != 9 { return 3 }

    0
}
