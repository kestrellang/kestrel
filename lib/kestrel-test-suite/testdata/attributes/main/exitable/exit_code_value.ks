// test: execution
// stdlib: true
// expect-exit: 2

// `@main` returning an `ExitCode` exits with that code.

module Main
import std.os.ExitCode

@main
func main() -> ExitCode { ExitCode(2) }
