// test: execution
// stdlib: true
// expect-exit: 3

// A user type conforming to `Exitable` works: the synthesized entry wrapper
// calls its `report()` witness.

module Main
import std.os.(Exitable, ExitCode)
import std.numeric.UInt8

struct Status: Exitable {
    var code: UInt8
    consuming func report() -> ExitCode { ExitCode(self.code) }
}

@main
func main() -> Status { Status(code: 3) }
