// test: diagnostics
// stdlib: true

// Same shape as `call_then_call`, but with two `print` calls — the form a
// user is most likely to type. The first call still needs its `;`.

module Test

func run() {
    print("a") // ERROR: expected `;`
    print("b");
}
