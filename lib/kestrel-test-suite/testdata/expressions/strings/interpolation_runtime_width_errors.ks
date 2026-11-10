// test: diagnostics
// stdlib: true

// A nested width hole must name an `Int64` value; an expression is E708.
module Test

func test() {
    let w = "x";
    let a = "[\(42:>{w})]"; // ERROR: type mismatch
    let b = "[\(42:>{nope})]"; // ERROR: undefined name 'nope'
    let c = "[\(42:>{1+2})]"; // ERROR: invalid format specifier: '{1+2}' must name a value
}
