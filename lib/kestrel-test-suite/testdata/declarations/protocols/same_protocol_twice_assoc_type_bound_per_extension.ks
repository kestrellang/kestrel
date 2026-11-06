// test: diagnostics
// stdlib: true

module Test

// A type conforming to one protocol twice binds a different unqualified
// `Output` in each sibling extension (datetime's `Instant - Duration ->
// Instant` and `Instant - Instant -> Duration`). Each conformance must read
// the `Output` its own extension declares, not the first one found.

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
