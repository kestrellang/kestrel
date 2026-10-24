// test: diagnostics
// stdlib: true

module Test

import std.numeric.Int64

// A plain Copyable type with a deinit would run it once per implicit copy
// (double-close). The compiler must reject it.
struct L {
    var fd: Int64;
    deinit { } // ERROR: Copyable type 'L' cannot have a deinit
}

// Move-only types may have a deinit.
struct Owned: not Copyable {
    var fd: Int64;
    deinit { }
}

// A non-Copyable field already makes the container move-only.
struct Holder {
    var o: Owned;
    deinit { }
}
