// test: diagnostics
// stdlib: true

module Main
import std.numeric.Int64
import std.result.Optional
func test() {
    let x: Optional[Optional[Int64]] = 5; // ERROR: type mismatch
}
