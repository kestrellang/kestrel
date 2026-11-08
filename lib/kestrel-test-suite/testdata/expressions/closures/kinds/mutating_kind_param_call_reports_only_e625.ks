// test: diagnostics
// stdlib: false

// A borrowing parameter of `mutating` closure type gets E625 at the
// signature. Calling it must not add an E203 per call ("it is bound with
// 'let'"): there is no `let`, and E625 already names the fix.
module Test

func apply(f: mutating () -> lang.i64) -> lang.i64 { // ERROR(E625)
    f();
    f()
}
