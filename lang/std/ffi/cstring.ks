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
import std.num.(Int64, UInt8)
import std.memory.(Pointer, RawPointer)
import std.core.(Convertible, Bool)
import std.text.(String)

/// A null-terminated C string suitable for FFI.
///
/// CString is an unsafe, non-owning wrapper around a pointer to null-terminated bytes.
/// It does NOT manage memory - the caller is responsible for ensuring the pointer
/// remains valid for the lifetime of the CString and for freeing memory when appropriate.
///
/// Conforms to FFISafe, so it can be used as a parameter or return type
/// in @extern(.C) function declarations.
///
/// # Safety
///
/// - The pointer must remain valid for the lifetime of the CString
/// - The pointer must point to null-terminated memory
/// - The length must match the actual string length (excluding null terminator)
/// - Caller is responsible for memory allocation and deallocation
public struct CString: FFISafe {
    /// The underlying pointer to the null-terminated bytes.
    public var raw: Pointer[UInt8]

    /// Creates a CString from a pointer.
    ///
    /// WARNING: This is unsafe. The caller is responsible for:
    /// - Ensuring the pointer is valid and remains valid
    /// - Ensuring the pointer points to null-terminated memory
    /// - Managing the memory lifecycle (allocation and deallocation)
    public init(raw rawPtr: Pointer[UInt8]) {
        self.raw = rawPtr;
    }

    /// Returns true if this CString's pointer is null.
    public var isNull: Bool {
        self.raw.isNull
    }

    /// The length of the string in bytes (not including the null terminator).
    /// Computed by scanning for the null terminator.
    public var length: Int64 {
        if self.raw.isNull {
            return 0
        }
        var len = Int64(intLiteral: 0);
        while self.raw.offset(by: len).read() != UInt8(intLiteral: 0) {
            len = len + 1;
        }
        len
    }

    /// Frees the memory pointed to by this CString.
    ///
    /// WARNING: This is unsafe. Only call this if you allocated the memory
    /// yourself (e.g., via toCString()). Do not call on CStrings wrapping
    /// external pointers unless you know it's safe to free them.
    ///
    /// After calling free(), the CString should not be used.
    public func free() {
        if not self.raw.isNull {
            std.ffi.free(self.raw.asRaw().raw);
        }
    }
}

// ============================================================================
// EXTENSION: String conversion helpers for FFI
// ============================================================================

/// Creates a CString from this String by allocating memory.
///
/// WARNING: This allocates memory using malloc that is NOT automatically freed.
/// The caller is responsible for freeing the memory when done using cstr.free().
///
/// This is an unsafe operation - use with care to avoid memory leaks.
///
/// Example:
///   let s = "Hello, C!"
///   let cstr = s.toCString()
///   puts(cstr)
///   cstr.free()
extend String {
    /// Creates a CString with a null-terminated copy of this string.
    /// Caller must free the memory: cstr.free()
    public func toCString() -> CString {
        let byteCount = self.byteCount;
        let totalSize = byteCount + Int64(intLiteral: 1); // +1 for null terminator

        // Allocate memory
        let rawPtr: lang.ptr[lang.i8] = malloc(totalSize.raw);
        let ptr = Pointer(raw: lang.cast_ptr[_, UInt8](rawPtr));

        // Copy bytes if allocation succeeded
        if ptr.isNull == false {
            // Copy bytes from source to destination if there are bytes to copy
            if byteCount > 0 {
                let srcPtr: lang.ptr[lang.i8] = self.bytes.asRaw();
                let _ = memcpy(rawPtr, srcPtr, byteCount.raw);
            }

            // Write null terminator
            ptr.offset(by: byteCount).write(UInt8(intLiteral: 0));
        }

        return CString(raw: ptr);
    }
}

// ============================================================================
// EXTENSION: String conforms to Convertible[CString]
// ============================================================================

/// String can be explicitly converted from a CString.
extend String: Convertible[CString] {
    /// Creates a String from a CString by copying the bytes.
    ///
    /// This copies the null-terminated bytes from the CString into a new
    /// Kestrel String (excluding the null terminator).
    ///
    /// Example:
    ///   let cstr = CString(raw: somePtr, length: Int64(intLiteral: 5))
    ///   let s = String(from: cstr)
    public init(from cstring: CString) {
        if cstring.length == 0 {
            self.init();
        } else {
            // Copy bytes from CString (excluding null terminator)
            self = String.fromBytesUnchecked(cstring.raw, cstring.length);
        }
    }
}
