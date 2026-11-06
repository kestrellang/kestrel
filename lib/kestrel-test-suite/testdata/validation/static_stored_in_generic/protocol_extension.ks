// test: diagnostics
// stdlib: false

module Test

// A protocol extension's `Self` is generic over every conformer: no stored
// statics there either.

protocol Named {}

extend Named {
    static var count: lang.i64 = 0; // ERROR: static stored properties not supported in generic types
}
