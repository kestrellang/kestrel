// test: diagnostics
// stdlib: false

// Two `@platform`s used to mean "all of", which can never match. The
// attribute is not repeatable; its argument list is the any-of set.

module Test

@platform(.Linux)
@platform(.Darwin) // ERROR: '@platform' appears twice; list every platform in one '@platform(...)'
func never() {}
