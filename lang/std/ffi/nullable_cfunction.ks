// NullableCFunction: a C function pointer that may be NULL, for C APIs whose
// callbacks are optional.

module std.ffi

import std.core.(Bool, ExpressibleByNullLiteral)
import std.memory.(Pointer, RawPointer)
import std.result.(Optional)

/// A C function of type `F` (`@extern(.C) (…) -> …`), or C's `NULL`.
///
/// A C function type is never null, so a C API whose callback is optional
/// takes a `NullableCFunction` instead: an `FFISafe` struct field or
/// `@extern` parameter of this type has exactly the layout of a C function
/// pointer, and the `null` literal is `NULL`.
///
/// # Examples
///
/// ```
/// @export(.C, mangleName: "on_log")
/// func onLog(level: Int32, message: CString) { ... }
///
/// var hooks = Hooks(log: NullableCFunction(onLog), panic: null)
/// if let .Some(log) = hooks.log.function {
///     unsafe log(1, message)
/// }
/// ```
///
/// `F` must be a C function type (E646).
@builtin(.NullableCFunctionStruct)
public struct NullableCFunction[F]: FFISafe, ExpressibleByNullLiteral where F: not Static {
    fileprivate var _raw: RawPointer

    /// @name From Function
    /// `f`, never null.
    public init(f: F) {
        // A C function value is one address: reinterpret its bits.
        self._raw = unsafe Pointer(to: f).cast[RawPointer]().read()
    }

    /// @name Null Literal
    /// C's `NULL` (`null`), for a callback left unset.
    public init() {
        self._raw = RawPointer.nullPointer()
    }

    /// Whether this is `null`.
    public var isNull: Bool { self._raw.isNull }

    /// The C function, or `None` when this is `null`.
    public var function: F? {
        if self._raw.isNull {
            return .None
        }
        let raw = self._raw;
        .Some(unsafe Pointer(to: raw).cast[F]().read())
    }

    /// The address as an untyped pointer.
    public var pointer: RawPointer { self._raw }
}
