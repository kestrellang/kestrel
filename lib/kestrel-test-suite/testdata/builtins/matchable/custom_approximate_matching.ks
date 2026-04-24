// test: diagnostics
// stdlib: false

module Test
struct ApproxFloat: Prelude.Matchable {
    var value: lang.f64
    var epsilon: lang.f64

    func matches(other: ApproxFloat) -> lang.i1 {
        // Check if values are within epsilon of each other
        let diff = lang.f64_sub(self.value, other.value);
        let absDiff = if lang.f64_lt(diff, 0.0) {
            lang.f64_neg(diff)
        } else {
            diff
        };
        lang.f64_le(absDiff, self.epsilon)
    }
}
