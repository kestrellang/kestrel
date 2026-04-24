// test: diagnostics
// stdlib: false

module Test
struct Coordinate: Prelude.Matchable {
    var x: lang.i64
    var y: lang.i64
    var z: lang.i64

    func matches(other: Coordinate) -> lang.i1 {
        // Only match on x and y, ignore z
        lang.i1_and(
            lang.i64_eq(self.x, other.x),
            lang.i64_eq(self.y, other.y)
        )
    }
}
