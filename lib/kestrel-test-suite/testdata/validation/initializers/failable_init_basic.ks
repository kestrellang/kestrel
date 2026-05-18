// test: diagnostics
// stdlib: true

module Test

struct Wrapper {
    var value: std.numeric.Int64

    init(from source: std.numeric.Int64)? {
        if source == 0 {
            return null
        }
        self.value = source
    }
}
