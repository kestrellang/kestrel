// test: diagnostics
// stdlib: false

// Stage 0.5: `&` does not exist in expression position — borrowing is
// decided by the callee's signature, never spelled at the call site. The
// parser recovers (prefix `&` parses as a unary op) so the LSP keeps a
// tree; HIR lowering rejects it.
module Test

func takesBorrow(x: lang.i64) { }

func f() {
    let y: lang.i64 = 1;
    takesBorrow(&y); // ERROR(E488)
}
