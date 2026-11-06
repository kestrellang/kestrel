// test: diagnostics

// A global initializer cannot read its own global, directly or through
// other globals. It used to read the zeroed storage (`g` printed 0).

module Test

let g: Int64 = g; // ERROR(E633): global initializer depends on itself: g -> g

let x: Int64 = y; // ERROR(E633): global initializer depends on itself: x -> y -> x
let y: Int64 = x + 1;
