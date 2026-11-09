// test: diagnostics
// stdlib: true

// An operator on a type without its protocol reports the conformance once.
// It used to add "no member 'subtract'" (the operator's hidden method name).
module Test

struct S {}

func f() {
    let a = S() - S(); // ERROR(E107): S !: Subtractable
    let b = -S(); // ERROR(E107): S !: Negatable
}
