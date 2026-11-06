// test: execution
// stdlib: true

// `IoError`: kinds from errno, the static constructors, descriptions and
// formatting.

module Test

import std.streams.(IoError, IoErrorKind)

@main
func main() -> lang.i64 {
    // From errno: the kind is named, the code kept.
    let two: Int32 = 2;
    let nf = IoError(errno: two);
    if nf.kind != IoErrorKind.NotFound { return 1 }
    if nf.errno != .Some(two) { return 2 }
    if nf.description() != "not found" { return 3 }

    // Static constructors carry no errno.
    if IoError.notFound.kind != IoErrorKind.NotFound { return 4 }
    if IoError.notFound.errno.isSome() { return 5 }
    if IoError.permissionDenied.description() != "permission denied" { return 6 }
    if IoError.alreadyExists.description() != "already exists" { return 7 }
    if IoError.invalidInput.description() != "invalid input" { return 8 }
    if IoError.wouldBlock.description() != "operation would block" { return 9 }
    if IoError.interrupted.description() != "interrupted" { return 10 }
    if IoError.brokenPipe.description() != "broken pipe" { return 11 }
    if IoError.timedOut.kind != IoErrorKind.TimedOut { return 12 }

    // An unnamed errno is `.Other` and shows the system message and code.
    let code999: Int32 = 999;
    let unknown = IoError(errno: code999);
    if unknown.kind != IoErrorKind.Other { return 13 }
    if not unknown.description().ends(with: "(errno 999)") { return 14 }

    // Formatting goes through description(), with width honored.
    if nf.formatted() != "not found" { return 15 }
    if "err: \(nf)" != "err: not found" { return 16 }
    if "\(nf:>30)".chars.count != 30 { return 17 }

    // Equal when kind and code match.
    if IoError(errno: two) != nf { return 18 }
    if IoError.notFound == nf { return 19 }
    0
}
