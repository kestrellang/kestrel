// test: execution
// stdlib: true
// expect-exit: -1
// expect-stdout: before abort\n

module Test

import std.system.(abort)

// `abort` raises SIGABRT, which skips `atexit`, so buffered standard output
// was lost. The streams shim installs a one-shot handler for fatal signals
// that flushes standard output and then re-raises the signal.
@main
func main() {
    println("before abort");
    abort()
}
