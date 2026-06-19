// test: execution
// stdlib: true
// expect-exit: -1

// Regression (#205): `fatalError(message)` must (a) trap at runtime — the
// process is killed by a signal, which the harness records as exit code -1
// (`status.code()` is `None` for signal termination) — and (b) accept an
// *interpolated* message. The interpolated form is the load-bearing part: it
// guards the `emit_panic` OSSA fix, where the diverging `lang.panic()`
// intrinsic now drops the message's formatting temps before the `Panic`
// terminator. Without that fix this body fails OSSA verification ("owned
// value live at block exit but never consumed") and never compiles.
//
// The message is written to stderr by the stdlib (`eprintln`); the harness
// only inspects stdout and the exit code, so the text is exercised here but
// not asserted.

module Test

@main
func main() {
    fatalError("regression #205: \(7 * 6)");
}
