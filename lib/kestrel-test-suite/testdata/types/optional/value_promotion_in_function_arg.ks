// test: diagnostics
// stdlib: true

module Main
import std.numeric.Int64
func takesOptional(x: Int64?) {}
func test() {
    takesOptional(5);
}
