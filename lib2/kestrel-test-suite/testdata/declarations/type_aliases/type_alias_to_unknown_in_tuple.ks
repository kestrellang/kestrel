// test: diagnostics
// stdlib: false

module Test

type Foo = (lang.i64, Unknown, lang.str) // ERROR: cannot find type
