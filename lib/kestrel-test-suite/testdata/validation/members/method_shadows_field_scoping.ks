// test: diagnostics
// stdlib: true

// E467 scoping (#130): a zero-arg EXTENSION method colliding with a stored
// field is flagged (in-body collisions are already E475) — but an extension
// method that REQUIRES labeled arguments (the stdlib's `slice` field +
// `slice(from:to:)` pattern) is disambiguated by its labels at every use
// site and must NOT be flagged, and neither may a static method (different
// namespace than `self.field`).

module Test

struct Box {
    let value: Int64
    let tag: Int64
    let kind: Int64
}

extend Box {
    func value() -> Int64 { 0 } // ERROR: method 'value' shadows the stored field 'value' of 'Box'

    // Requires a labeled argument — no collision with bare `self.tag`.
    func tag(scaled by: Int64) -> Int64 { by }

    // Static — resolved via the type, not `self.kind`.
    static func kind() -> Int64 { 7 }
}

@main
func main() {
    // The zero-arg collision is also ambiguous at the use site; the labeled
    // and static methods are not exercised here (use-site resolution of a
    // field/labeled-method pair is a separate, pre-existing concern).
    let b = Box(value: 1, tag: 2, kind: 3);
    println("\(b.value)") // ERROR: ambiguous member 'value'
}
