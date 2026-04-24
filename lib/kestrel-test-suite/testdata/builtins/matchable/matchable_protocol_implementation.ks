// test: diagnostics
// stdlib: false

module Test
struct CaseInsensitiveChar: Prelude.Matchable {
    var char: lang.i64

    func matches(other: CaseInsensitiveChar) -> lang.i1 {
        // Simplified: just compare the values
        lang.i64_eq(self.char, other.char)
    }
}
func useMatchable(a: CaseInsensitiveChar, b: CaseInsensitiveChar) -> lang.i1 {
    a.matches(b)
}
