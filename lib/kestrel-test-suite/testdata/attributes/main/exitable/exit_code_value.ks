// test: execution
// stdlib: true
// expect-exit: 2

// `@main` returning an `ExitCode` exits with that code.

module Main
import std.system.ExitCode

@main
func main() -> ExitCode { ExitCode(2) }
