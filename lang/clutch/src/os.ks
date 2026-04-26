// OS-level bindings for CLI tools
//
// Re-exports general OS operations from std.os and provides
// CLI-specific functionality like command-line argument access.

module clutch.os

// std.os functions (fileExists, isDirectory, listDir, getcwd, getenv, spawn, captureOutput)
// are auto-imported from stdlib

// ============================================================================
// RAW FFI BINDINGS (CLI-specific, macOS)
// ============================================================================

// Command-line arguments (macOS)
@platform(.darwin)
@extern(.C, mangleName: "_NSGetArgc")
func nsGetArgc() -> lang.ptr[lang.i32]

@platform(.darwin)
@extern(.C, mangleName: "_NSGetArgv")
func nsGetArgv() -> lang.ptr[lang.ptr[lang.ptr[lang.i8]]]

// ============================================================================
// PUBLIC API
// ============================================================================

/// Returns the command-line arguments as an array of strings.
/// Skips argv[0] (the program name).
@platform(.darwin)
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
        let cstr = CString(raw: Pointer(raw: lang.cast_ptr[_, UInt8](argPtr)));
        let s = String(from: cstr);
        result.append(s);
        i = i + 1
    }

    result
}

/// Returns the command-line arguments as an array of strings.
/// Skips argv[0] (the program name).
@platform(.linux)
public func getArgv() -> Array[String] {
    var result = Array[String]();

    // Open /proc/self/cmdline using stdlib io functions
    let path = "/proc/self/cmdline".toCString();
    let fd = std.io.libc.open(path.raw, O_RDONLY(), 0);
    path.free();
    if fd < 0 {
        return result
    }

    // Read the entire cmdline (args are null-separated)
    var buf = Array[UInt8](capacity: 4096);
    var i: Int64 = 0;
    while i < 4096 {
        buf.append(0);
        i = i + 1
    }
    let n = std.io.libc.read(fd, buf.asPointer(), 4096);
    let _ = std.io.libc.close(fd);

    if n <= 0 {
        return result
    }

    // Parse null-separated arguments, skip first (program name)
    var argStart: Int64 = 0;
    var argIndex: Int64 = 0;
    var pos: Int64 = 0;
    while pos < n {
        if buf(pos) == 0 {
            if argIndex > 0 {
                // Build string from argStart to pos
                var s = String();
                var j = argStart;
                while j < pos {
                    s.appendByte(buf(j));
                    j = j + 1
                }
                result.append(s);
            }
            argIndex = argIndex + 1;
            argStart = pos + 1
        }
        pos = pos + 1
    }

    result
}
