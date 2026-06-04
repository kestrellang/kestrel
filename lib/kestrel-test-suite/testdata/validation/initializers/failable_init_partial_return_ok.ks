// test: diagnostics
// stdlib: true
//
// Returning null in a failable init with only some fields initialized
// should NOT produce E009 — it's a failure path.

module Test

struct Pair {
    var x: std.numeric.Int64
    var y: std.numeric.Int64

    init(from source: std.numeric.Int64)? {
        self.x = source
        if source == 0 {
            return null
        }
        self.y = source
    }
}
