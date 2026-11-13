// test: execution
// stdlib: true
// expect-exit: 1

// `ExitCode.failure` is the conventional generic-failure code, 1.

module Main
import std.system.ExitCode

@main
func main() -> ExitCode { ExitCode.failure }
