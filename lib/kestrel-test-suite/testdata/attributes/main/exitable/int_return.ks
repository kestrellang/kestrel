// test: execution
// stdlib: true
// expect-exit: 42

// Stdlib `Int64` conforms to `Exitable`: a returned integer is its own exit
// code (low 8 bits). Previously rejected by E616; now valid.

module Main
import std.numeric.Int64

@main
func main() -> Int64 { 42 }
