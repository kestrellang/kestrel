// test: diagnostics
// stdlib: true

module Main
import std.numeric.Int64
import std.core.Bool
func test(cond: Bool) -> Int64? {
    let x = if cond { 5 } else { 10 };
    x
}
