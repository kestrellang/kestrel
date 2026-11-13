// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.streams.(IoErrorKind)

// A bare `.Case` on the left of a comparison takes its type from the right
// operand, the same as `x == .Case` takes it from the parameter.
@derive(Equatable)
enum Level {
    case Low
    case High
}

@main
func main() -> lang.i64 {
    let kind = IoErrorKind.TimedOut;
    if not (.TimedOut == kind) { return 1 }
    if .NotFound == kind { return 2 }
    if not (.NotFound != kind) { return 3 }
    let level = Level.High;
    if not (.High == level) { return 4 }
    // Through an Optional, as on the right (`.Some` / `.None`).
    let maybe: Level? = .Some(.Low);
    if not (.Some(.Low) == maybe) { return 5 }
    if .None == maybe { return 6 }
    0
}
