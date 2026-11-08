// test: diagnostics
// stdlib: false

module Test
struct Blob {}
@fileconstant(42) // ERROR: expected a string literal, found '42'
let table: Blob = Blob()
