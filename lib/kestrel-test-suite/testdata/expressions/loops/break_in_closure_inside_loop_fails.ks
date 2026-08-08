// test: diagnostics
// stdlib: false

// A closure body is a separate function body, so `break` in it cannot target a
// loop in the enclosing body. hir-lower used to leave the enclosing loop stack
// visible while lowering a closure, so `validate_break_continue` accepted this
// and MIR's `lower_break` then found no loop and emitted a unit literal — the
// `break` compiled to a silent no-op.

module Main

func test() {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 3) {
        let f: () -> () = { () in
            break; // ERROR: outside of loop
        };
        i = lang.i64_add(i, 1);
    }
}
