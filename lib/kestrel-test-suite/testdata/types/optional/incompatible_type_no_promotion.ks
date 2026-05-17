// test: diagnostics
// stdlib: true

module Main
import std.numeric.Int64
import std.text.String
func test() {
    let x: String? = 5; // ERROR: type mismatch
}
