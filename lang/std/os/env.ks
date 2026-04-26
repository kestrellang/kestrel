// Environment variable access

module std.os.env

import std.num.(UInt8)
import std.memory.(Pointer)
import std.text.(String)
import std.core.(Bool)
import std.ffi.(CString)
import std.result.(Optional)

// ============================================================================
// RAW FFI BINDINGS
// ============================================================================

@extern(.C, mangleName: "getenv")
func libc_getenv(name: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

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
    let result = libc_getenv(lang.cast_ptr[_, lang.i8](cname.raw.raw));
    cname.free();

    if Bool(boolLiteral: lang.ptr_is_null(result)) {
        return .None
    }

    let cstr = CString(raw: Pointer(raw: lang.cast_ptr[_, UInt8](result)));
    .Some(String(from: cstr))
    // Note: do not free result - it points to environ memory
}
