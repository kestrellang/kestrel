// Environment variable access

module std.os

import std.numeric.(UInt8)
import std.memory.(Pointer, RawPointer)
import std.text.(String)
import std.core.(Bool)
import std.ffi.(CString)
import std.result.(Optional)

// ============================================================================
// RAW FFI BINDINGS
// ============================================================================

@extern(.C, mangleName: "getenv")
func libc_getenv(name: RawPointer) -> RawPointer

// ============================================================================
// PUBLIC API
// ============================================================================

/// Looks up the value of the environment variable `name`.
///
/// Returns `Some(value)` if the variable is set (including the empty
/// string), `None` if it is unset. Wraps `libc::getenv`, which returns
/// a pointer into the `environ` block — this function copies the bytes
/// into a Kestrel `String` immediately, so the result is safe to keep
/// across subsequent `setenv` / `unsetenv` calls.
///
/// # Examples
///
/// ```
/// match getenv(name: "HOME") {
///     .Some(path) => print(path),
///     .None      => print("HOME not set")
/// }
/// ```
public func getenv(name: String) -> Optional[String] {
    let cname = name.toCString();
    let result = libc_getenv(cname.raw.asRaw());
    cname.free();

    if result.isNull {
        return .None
    }

    let cstr = CString(raw: result.cast[UInt8]());
    .Some(String(from: cstr))
}
