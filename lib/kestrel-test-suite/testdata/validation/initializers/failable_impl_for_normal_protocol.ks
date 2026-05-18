// test: diagnostics
// stdlib: true

module Test

import std.numeric.Int64

protocol Constructible {
    init(from source: Int64)
}

struct Bad: Constructible {
    var value: Int64

    init(from source: Int64)? { // ERROR
        if source < 0 { return null }
        self.value = source
    }
}
