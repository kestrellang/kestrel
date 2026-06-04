// test: execution
// stdlib: true
// expect-exit: -1

// Force-unwrapping `.None` must trap at runtime (not return garbage).
// `forceUnwrap()` on `.None` calls `fatalError`, which lowers to a
// `Panic` terminator → a cranelift trap → the process is killed by a
// signal (SIGILL). The harness records a signal-kill as exit code -1
// (`status.code()` is `None` for signal termination, see runner.rs).

module Test

func main() -> lang.i64 {
    let opt: std.result.Optional[std.numeric.Int64] = .None;
    let v = opt!;            // traps here
    if v != 0 { return 1 }   // unreachable
    0
}
