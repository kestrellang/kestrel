// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `a - Meters(..)` and `a - Stamp(..)` dispatch to sibling conformances of
// `Subtractable`, each with its own `Output`. The operator's result type must
// come from the conformance the argument selects.

struct Meters { var v: Int64 }
struct Stamp { var t: Int64 }

extend Stamp: Subtractable[Meters] {
    type Output = Stamp
    public consuming func subtract(consuming other: Meters) -> Stamp { Stamp(t: self.t - other.v) }
}

extend Stamp: Subtractable[Stamp] {
    type Output = Meters
    public consuming func subtract(consuming other: Stamp) -> Meters { Meters(v: self.t - other.t) }
}

@main
func main() -> Int64 {
    let a = Stamp(t: 10);
    let b = a - Meters(v: 3);
    let d = a - Stamp(t: 4);
    if b.t != 7 { return 1 }
    if d.v != 6 { return 2 }
    0
}
