// test: diagnostics
// stdlib: false

module Test
struct Error {}
throw Error() // ERROR: found 'throw'
