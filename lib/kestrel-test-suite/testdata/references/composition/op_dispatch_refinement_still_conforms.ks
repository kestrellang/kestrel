// test: execution
// backends: cranelift,llvm
// stdlib: true

// The clean-reject fix gates INDIRECT conformance supply (refinement
// parents like `protocol Comparable: Equatable`, and blanket extensions
// on protocols like `extend Equatable: Equal[Self]`) on the receiver
// genuinely satisfying the parent protocol. The positive direction must
// keep working: Version's Equatable arrives ONLY via the Comparable
// refinement (decl-direct), and ==/!=/< all chain through blankets.
module Test

struct Version: Comparable {
    var major: Int64

    func isEqual(to other: Version) -> Bool {
        self.major == other.major
    }

    func compare(other: Version) -> Ordering {
        self.major.compare(other.major)
    }
}

@main
func main() -> lang.i64 {
    let a = Version(major: 3);
    let b = Version(major: 3);
    if a == b { } else { return 1; }
    if a != Version(major: 4) { } else { return 2; }
    if a < Version(major: 9) { } else { return 3; }
    if Version(major: 9) > a { } else { return 4; }
    0
}
