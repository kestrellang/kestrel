// test: diagnostics
// stdlib: true

// Expression statement followed by `return`. `return` is statement-like for
// the *inline* expression list but not for a function-body block item, so the
// preceding call still needs its `;`.

module Test

func f() -> lang.i64 {
    print("hi") // ERROR: expected `;`
    return 3;
}
