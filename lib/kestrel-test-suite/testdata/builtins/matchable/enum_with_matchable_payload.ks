// test: diagnostics
// stdlib: false

module Test
struct Version: Prelude.Matchable {
    var major: lang.i64
    var minor: lang.i64

    func matches(other: Version) -> lang.i1 {
        // Only match on major version
        lang.i64_eq(self.major, other.major)
    }
}
enum Software {
    case App(Version)
    case Library(Version)
}
