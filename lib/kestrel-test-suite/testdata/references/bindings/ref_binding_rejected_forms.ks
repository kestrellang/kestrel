// test: diagnostics
// stdlib: false

// Named ref bindings (stage 1.5 item 2) are `let`-only with a single
// name: `var r = &x` would suggest rebinding (which would need a deref
// spelling Kestrel doesn't have), and destructuring would alias the
// desugared temp. Both reject with E209 and recover by dropping the `&`.
module Test

func f() {
    var x: lang.i64 = 1;
    var r = &x; // ERROR(E209)
    var t = (1, 2);
    let (a, b) = &t; // ERROR(E209)
}
