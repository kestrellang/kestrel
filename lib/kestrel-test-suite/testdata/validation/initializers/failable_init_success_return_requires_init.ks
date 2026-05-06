// test: diagnostics
// stdlib: true
//
// Bare return (success path) in a failable init still requires all fields initialized.

module Test

struct Pair {
    var x: std.numeric.Int64
    var y: std.numeric.Int64

    init(from source: std.numeric.Int64)? {
        self.x = source
        return // ERROR: cannot return before all fields
    }
}
