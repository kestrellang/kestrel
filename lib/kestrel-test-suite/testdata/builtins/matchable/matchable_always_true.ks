// test: diagnostics
// stdlib: false

module Test
struct Wildcard: Prelude.Matchable {
    var ignored: lang.i64

    func matches(other: Wildcard) -> lang.i1 {
        true
    }
}
