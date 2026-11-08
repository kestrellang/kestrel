// test: diagnostics
// stdlib: false

module Test
struct Blob {}
@fileconstant // ERROR: '@fileconstant' needs a path
let table: Blob = Blob()
