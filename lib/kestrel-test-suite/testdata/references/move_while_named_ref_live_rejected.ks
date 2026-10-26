// test: diagnostics

// Consuming a value while a named reference into it is live is E498. It used
// to reach the MIR verifier as an internal "active borrow" error.

module Test

struct R: not Copyable { var id: Int64; }
func take(consuming r: R) { }

func moved() -> Int64 {
    let a = R(id: 1);
    let r = &a;
    take(a); // ERROR(E498): cannot consume `a` while a reference into it is live
    r.id
}
