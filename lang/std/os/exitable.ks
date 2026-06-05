// Process exit codes and the `Exitable` protocol for `@main` return types.
//
// A `@main` function may return any `Exitable` type. The compiler synthesizes
// the real C `main` as a wrapper that calls `report()` on whatever `@main`
// returns and uses the resulting `ExitCode` as the process exit status.

module std.os

import std.numeric.(Int8, Int16, Int32, Int64, UInt8, UInt16, UInt32, UInt64)
import std.text.(Formattable)
import std.result.(Result)
import std.io.(eprintln)

/// A process exit code.
///
/// Conventionally `0` means success and any non-zero value means failure. Only
/// the low 8 bits survive on POSIX (`WEXITSTATUS`), so the meaningful range is
/// `0`–`255`. The byte is private; build one with `ExitCode(_:)` or use the
/// `.success` / `.failure` constants.
@builtin(.ExitCode)
public struct ExitCode {
    // Signed `lang.i8` backing: a returned code sign-extends into the C `int`
    // exactly the way `exit(-1)` truncates to 255. The synthesized `@main`
    // wrapper reads this field directly.
    private var rawValue: lang.i8

    /// Builds an exit code from a byte. `exit(-1)`-style codes are spelled
    /// `ExitCode(255)`.
    public init(value: UInt8) {
        self.rawValue = value.raw
    }

    /// The conventional success code, `0`.
    public static var success: ExitCode { ExitCode(0) }

    /// The conventional generic-failure code, `1`.
    public static var failure: ExitCode { ExitCode(1) }
}

/// A type that a `@main` function may return: it knows how to produce a process
/// exit code.
///
/// The compiler synthesizes C `main` as a wrapper that calls `report()` on the
/// value `@main` returns. `report()` is `consuming` so move-only conformers
/// (e.g. `Result`) can move out their payload.
@builtin(.Exitable)
public protocol Exitable {
    /// Produce the process exit code for this value.
    @builtin(.ExitableReport)
    consuming func report() -> ExitCode
}

extend ExitCode: Exitable {
    consuming func report() -> ExitCode { self }
}

// ============================================================================
// Integer conformances — a returned integer is its own exit code (low 8 bits).
// Declared here (retroactively) so std.numeric needn't depend on std.os.
// ============================================================================

extend Int8: Exitable {
    consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) }
}
extend Int16: Exitable {
    consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) }
}
extend Int32: Exitable {
    consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) }
}
extend Int64: Exitable {
    consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) }
}
extend UInt8: Exitable {
    consuming func report() -> ExitCode { ExitCode(self) }
}
extend UInt16: Exitable {
    consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) }
}
extend UInt32: Exitable {
    consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) }
}
extend UInt64: Exitable {
    consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) }
}

// ============================================================================
// Throwing `main`: `main() throws E` desugars to `Result[(), E]`. On `.Ok`
// the process exits 0; on `.Err` the error is printed to stderr and the
// process exits non-zero. Specialized on unit only (see exitable design doc).
// ============================================================================

extend Result[(), E]: Exitable where E: Formattable {
    consuming func report() -> ExitCode {
        match self {
            .Ok(_)      => ExitCode.success,
            .Err(error) => {
                let _ = eprintln(error);
                ExitCode.failure
            }
        }
    }
}
