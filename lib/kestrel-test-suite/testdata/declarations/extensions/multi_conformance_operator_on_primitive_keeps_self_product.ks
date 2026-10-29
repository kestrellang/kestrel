// test: execution
// stdlib: true
// expect-exit: 18

// A second conformance of a stdlib primitive to an operator protocol with a
// DIFFERENT argument (`extend Int64: Multipliable[Meters]`, alongside Int64's
// own `Multipliable[Int64]`) must not change what `Int64 * Int64` means.
// `a * b` must still pick `Multipliable[Int64]` (Output = Int64), and
// `n * Meters` must pick the new conformance (Output = Meters).
//
// Surfaced by the datetime package: adding `Int64 * Duration` made the solver
// type every `Int64 * Int64` in std's array.ks as `Duration` (E109/E107
// "expected Int64 got Duration", "Duration !: ExpressibleByIntLiteral"), so the
// package ships without that operator.
//
// Related: multi_conformance_same_protocol_return_disambiguated.ks (user-type
// receiver; this case is a primitive receiver with a pre-existing conformance).
module Main

struct Meters: Copyable {
    var value: Int64
}

extend Int64: Multipliable[Meters] {
    type Output = Meters

    public consuming func multiply(consuming other: Meters) -> Meters {
        Meters(value: self * other.value)
    }
}

@main
func main() -> Int64 {
    let a: Int64 = 2;
    let b: Int64 = 3;
    let area: Int64 = a * b;           // 6 via Multipliable[Int64]
    let scaled = 4 * Meters(value: 3);  // 12 via Multipliable[Meters]
    area + scaled.value                 // 6 + 12 = 18
}
