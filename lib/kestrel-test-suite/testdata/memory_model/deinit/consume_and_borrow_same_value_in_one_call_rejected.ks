// test: diagnostics

// One call cannot both borrow and consume the same non-Copyable value: the
// borrow lasts for the whole call. It used to reach the MIR verifier as an
// internal "active mut borrow" error.

module Test

struct R: not Copyable { var id: Int64; }
func f(mutating a: R, consuming b: R) { }

func run() {
    var x = R(id: 1);
    f(x, x); // ERROR(E510): cannot consume 'x' while the same call borrows it
}
