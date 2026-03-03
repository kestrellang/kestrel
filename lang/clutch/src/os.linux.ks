// OS-level bindings for CLI tools (Linux)
//
// Provides CLI-specific functionality like command-line argument access.
// Uses /proc/self/cmdline to read arguments on Linux.

module clutch.os

// std.os functions (fileExists, isDirectory, listDir, getcwd, getenv, spawn, captureOutput)
// are auto-imported from stdlib

// ============================================================================
// RAW FFI BINDINGS
// ============================================================================

@extern(.C, mangleName: "open")
func libc_open(path: lang.ptr[lang.i8], flags: lang.i32, mode: lang.i32) -> lang.i32

@extern(.C, mangleName: "read")
func libc_read(fd: lang.i32, buf: lang.ptr[lang.i8], count: lang.i64) -> lang.i64

@extern(.C, mangleName: "close")
func libc_close(fd: lang.i32) -> lang.i32

// ============================================================================
// PUBLIC API
// ============================================================================

/// Returns the command-line arguments as an array of strings.
/// Skips argv[0] (the program name).
public func getArgv() -> Array[String] {
    var result = Array[String]();

    // Open /proc/self/cmdline
    let pathBytes = "/proc/self/cmdline";
    let pathPtr = Pointer(to: pathBytes).cast[UInt8]();
    let fd = Int32(raw: libc_open(lang.cast_ptr[lang.i8](pathPtr.raw), 0, 0));
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
    let n = Int64(raw: libc_read(fd.raw, lang.cast_ptr[lang.i8](buf.asPointer().raw), 4096));
    let _ = Int32(raw: libc_close(fd.raw));

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
