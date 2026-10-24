// test: diagnostics
// stdlib: false

// An unknown platform used to be ignored silently, compiling the declaration
// everywhere. It is now an error, and the declaration is kept.

module Test

@platform(.Windows) // ERROR: '.Windows' is not a platform; expected '.Darwin', '.Linux', '.darwin' or '.linux'
func f() {}
