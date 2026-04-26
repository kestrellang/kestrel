// test: diagnostics
// stdlib: false

module Test
struct Number: Prelude.Matchable {
    var value: lang.i64

    func matches(other: Number) -> lang.i1 {
        lang.i64_eq(self.value, other.value)
    }
}
