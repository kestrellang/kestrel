// test: diagnostics
// stdlib: false

// E-REF-16: a ref-returning function is not a first-class value — the
// ret_borrow ABI is not expressible in function types, so capturing or
// storing one is a silent-miscompile backdoor and must be rejected.
module Test

struct Holder {
    var v: lang.i64
    func peek() -> &lang.i64 { self.v }
}

func free(x: lang.i64) -> &lang.i64 { x }

func use(h: Holder) {
    let f = free; // ERROR(E491)
}
