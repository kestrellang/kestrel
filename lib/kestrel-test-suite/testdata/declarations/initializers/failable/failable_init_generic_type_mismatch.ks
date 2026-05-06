// test: diagnostics
// stdlib: true

module Test

import std.numeric.Int64

protocol Parseable {
    init(from source: Int64)?
}

struct Wrapper: Parseable {
    var value: Int64

    init(from source: Int64)? {
        if source == 0 {
            return null
        }
        self.value = source
    }
}

// T(from:) returns T?, assigning to T should be a type error
func make[T](from source: Int64) -> T where T: Parseable {
    return T(from: source) // ERROR
}
