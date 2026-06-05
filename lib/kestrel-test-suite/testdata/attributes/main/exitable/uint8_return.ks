// test: execution
// stdlib: true
// expect-exit: 9

// `UInt8` conforms to `Exitable` too.

module Main
import std.numeric.UInt8

@main
func main() -> UInt8 { 9 }
