// test: execution
// stdlib: true
// expect-exit: 3

// exit takes an ExitCode, and an integer literal is one.

module Test

func fail() -> ! {
    std.system.exit(3)
}

@main
func main() -> lang.i64 {
    fail()
}
