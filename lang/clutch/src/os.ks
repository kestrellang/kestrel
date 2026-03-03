// OS-level bindings for CLI tools
//
// Re-exports general OS operations from std.os and provides
// CLI-specific functionality like command-line argument access.

module clutch.os

// std.os functions (fileExists, isDirectory, listDir, getcwd, getenv, spawn, captureOutput)
// are auto-imported from stdlib

// ============================================================================
// RAW FFI BINDINGS (CLI-specific)
// ============================================================================

// Command-line arguments (macOS)
@extern(.C, mangleName: "_NSGetArgc")
func nsGetArgc() -> lang.ptr[lang.i32]

@extern(.C, mangleName: "_NSGetArgv")
func nsGetArgv() -> lang.ptr[lang.ptr[lang.ptr[lang.i8]]]

// ============================================================================
// PUBLIC API
// ============================================================================

/// Returns the command-line arguments as an array of strings.
/// Skips argv[0] (the program name).
public func getArgv() -> Array[String] {
    var result = Array[String]();

    let argcPtr = nsGetArgc();
    let argc = Int32(raw: lang.ptr_read(argcPtr));

    let argvPtrPtr = nsGetArgv();
    let argvPtr: lang.ptr[lang.ptr[lang.i8]] = lang.ptr_read(argvPtrPtr);

    let argcInt = Int64(from: argc);
    var i: Int64 = 1; // skip program name
    while i < argcInt {
        let byteOffset = i * 8; // ptr size
        let argPtr: lang.ptr[lang.i8] = lang.ptr_read(lang.ptr_offset(argvPtr, byteOffset.raw));
        let cstr = CString(raw: Pointer(raw: lang.cast_ptr[UInt8](argPtr)));
        let s = String(from: cstr);
        result.append(s);
        i = i + 1
    }

    result
}
