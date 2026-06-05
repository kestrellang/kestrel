// CString - An unsafe null-terminated C string for FFI compatibility
//
// CString provides a way to work with C-compatible null-terminated strings.
// It is an unsafe, non-owning wrapper - the caller is responsible for all
// memory management (allocation and deallocation).
//
// The CString type:
// - Does NOT manage memory (no automatic allocation or freeing)
// - Conforms to FFISafe for use in FFI function signatures
// - Provides length property for the string content (excluding null terminator)
// - Caller must ensure pointer validity and manage lifecycle
//
// Example usage:
//   @extern(.C, mangleName: "puts")
//   func puts(s: CString) -> Int32
//
//   let message = "Hello, C!"
//   let cstr = message.toCString()
//   puts(cstr)
//   cstr.free()  // Caller must free!
//
//   let back = String(from: cstr)  // Convert back to String

module std.ffi

import std.ffi.(FFISafe, malloc, free, memcpy)
import std.numeric.(Int64, UInt8)
import std.memory.(Pointer, RawPointer)
import std.core.(Convertible, Bool)
import std.text.(String)

/// A null-terminated, non-owning byte pointer suitable for `@extern(.C)` boundaries.
///
/// `CString` is an FFI shim — it carries a `Pointer[UInt8]` that
/// the C side will treat as `const char *`, but it does **not** own
/// the memory. The pointer's lifetime, validity, and disposal are
/// entirely the caller's responsibility. Two common ownership
/// patterns: (1) the C side returns a pointer into static or
/// `environ` memory — wrap it in a `CString` and read, but never
/// free; (2) the Kestrel side allocates via `String.toCString()` —
/// the caller must `free()` the result.
///
/// # Examples
///
/// ```
/// @extern(.C, mangleName: "puts")
/// func puts(s: CString) -> Int32
///
/// let cstr = "Hello, C!".toCString();
///  puts(cstr);
/// cstr.free();
/// ```
///
/// # Safety
///
/// - The pointer must remain valid for as long as the `CString` is used.
/// - The pointed-to bytes must end with a `0` terminator.
/// - `length` is computed by scanning to the terminator — quadratic
///   if you build long strings by repeated reads of `length`.
/// - The caller chooses whether `free()` is appropriate (yes for
///   self-allocated, no for borrowed pointers).
///
/// # Representation
///
/// A single `Pointer[UInt8]` field. No length is cached.
///
/// # Memory Model
///
/// Non-owning. Conforms to `FFISafe` so it passes through
/// `@extern(.C)` signatures unchanged.
public struct CString: FFISafe {
    /// The underlying pointer to the null-terminated bytes.
    public var raw: Pointer[UInt8]

    /// @name From Pointer
    /// Wraps an existing pointer as a `CString`.
    ///
    /// Performs no validation — the caller affirms that the pointer
    /// is null or points at null-terminated memory.
    ///
    /// # Safety
    ///
    /// - `rawPtr` must be null or point at a null-terminated byte
    ///   sequence.
    /// - The pointed-to bytes must remain valid for the lifetime of
    ///   the `CString`.
    /// - The caller decides whether `free()` is later appropriate.
    public init(raw rawPtr: Pointer[UInt8]) {
        self.raw = rawPtr;
    }

    /// True if the wrapped pointer is null.
    ///
    /// A null `CString` should not be passed to a C function that
    /// expects a string; check this before calling.
    public var isNull: Bool {
        self.raw.isNull
    }

    /// Length of the string in bytes, **excluding** the null terminator.
    ///
    /// Computed by linear scan — O(n). Cache the result if you
    /// need it more than once. Returns `0` for a null pointer
    /// (defensive: avoids dereferencing).
    public var length: Int64 {
        if self.raw.isNull {
            return 0
        }
        var len = 0;
        while self.raw.offset(by: len).read() != 0 {
            len = len + 1;
        }
        len
    }

    /// Frees the buffer pointed to by this `CString` via libc `free`.
    ///
    /// No-op when the pointer is null. After this call the `CString`
    /// is dangling — do not read its bytes or call any other method
    /// that touches the pointer.
    ///
    /// # Safety
    ///
    /// Only call this on `CString`s whose pointer was produced by a
    /// prior `malloc` (e.g. via `String.toCString()`). Calling on a
    /// borrowed pointer (returned by `getenv`, a string literal,
    /// etc.) is undefined behaviour.
    public func free() {
        if not self.raw.isNull {
            std.ffi.free(self.raw.asRaw());
        }
    }
}

// ============================================================================
// EXTENSION: String conversion helpers for FFI
// ============================================================================

/// FFI conversion helpers on `String` — see `toCString()` for the full contract.
extend String {
    /// Allocates a fresh null-terminated copy of this string and returns it as a `CString`.
    ///
    /// Sizes the buffer to `byteCount + 1`, copies the source bytes
    /// via `memcpy`, and writes the trailing `\0`. The caller takes
    /// ownership and must release the buffer with `cstr.free()`.
    ///
    /// # Safety
    ///
    /// The returned `CString` aliases freshly allocated memory; do
    /// not pass it to a C function that takes ownership of the
    /// pointer (it will then be double-freed) and do not forget to
    /// free it.
    ///
    /// # Examples
    ///
    /// ```
    /// let cstr = "Hello, C!".toCString();
    ///  puts(cstr);
    /// cstr.free();
    /// ```
    public func toCString() -> CString {
        let byteCount = self.byteCount;
        let totalSize = byteCount + 1;

        let rawPtr = malloc(totalSize);
        let ptr = rawPtr.cast[UInt8]();

        if ptr.isNull == false {
            if byteCount > 0 {
                let srcPtr = RawPointer(raw: self.bytes.asRaw());
                 memcpy(rawPtr, srcPtr, byteCount);
            }

            ptr.offset(by: byteCount).write(0);
        }

        return CString(raw: ptr);
    }
}

// ============================================================================
// EXTENSION: String conforms to Convertible[CString]
// ============================================================================

/// String can be explicitly converted from a CString.
extend String: Convertible[CString] {
    /// @name From CString
    /// Builds a `String` by copying the bytes out of `cstring`, excluding the null terminator.
    ///
    /// O(n) — `cstring.length` walks to the terminator and the byte
    /// copy is linear. Empty `CString`s (length zero) yield the
    /// default empty `String` without touching the pointer.
    ///
    /// # Safety
    ///
    /// `cstring.raw` must be valid for at least `length` readable
    /// bytes plus a terminator. The conversion does not free the
    /// `CString`'s buffer — caller still owns it.
    ///
    /// # Examples
    ///
    /// ```
    /// let cstr = CString(raw: somePtr);
    /// let s = String(from: cstr);
    /// ```
    public init(from cstring: CString) {
        if cstring.length == 0 {
            self.init();
        } else {
            // Copy bytes from CString (excluding null terminator)
            self = String.fromBytesUnchecked(cstring.raw, cstring.length);
        }
    }
}
