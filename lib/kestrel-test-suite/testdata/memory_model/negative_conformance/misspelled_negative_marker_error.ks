// test: diagnostics
// stdlib: true

// A misspelled marker after `not` is an unknown type. It used to be dropped
// silently, leaving `H` Copyable.

module Test

struct H: not Copyabel { // ERROR: cannot find type 'Copyabel'
    var x: lang.i64
}
