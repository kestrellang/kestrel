/// Platform-specific access to the process's command-line arguments.
///
/// Provides `getArgv()`, which returns the raw `argv` tokens (minus the
/// program name) as an `Array[String]`. Separate implementations exist
/// for macOS (`_NSGetArgc` / `_NSGetArgv`) and Linux
/// (`/proc/self/cmdline`).

module clutch.os

// ============================================================================
// RAW FFI BINDINGS (macOS)
// ============================================================================

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
///
/// The program name (`argv[0]`) is skipped; the returned array starts
/// with the first user-supplied argument. Returns an empty array when
/// no arguments were passed.
///
/// # Examples
///
/// ```
/// // $ mycli --verbose input.txt
/// let args = getArgv();
/// args;  // ["--verbose", "input.txt"]
/// ```
@platform(.darwin)
public func getArgv() -> Array[String] {
    var result = Array[String]();

    let argcPtr = nsGetArgc();
    let argc = Int32(raw: lang.ptr_read(argcPtr));

    let argvPtrPtr = nsGetArgv();
    let argvPtr: lang.ptr[lang.ptr[lang.i8]] = lang.ptr_read(argvPtrPtr);

    let argcInt = Int64(from: argc);
    for i in 1..<argcInt {
        let byteOffset = i * 8; // ptr size
        let argPtr: lang.ptr[lang.i8] = lang.ptr_read(lang.ptr_offset(argvPtr, byteOffset.raw));
        let cstr = CString(raw: Pointer(raw: lang.cast_ptr[_, UInt8](argPtr)));
        let s = String(from: cstr);
        result.append(s);
    }

    result
}

/// Returns the command-line arguments as an array of strings.
///
/// The program name (`argv[0]`) is skipped; the returned array starts
/// with the first user-supplied argument. Returns an empty array when
/// no arguments were passed or `/proc/self/cmdline` cannot be read.
///
/// # Examples
///
/// ```
/// // $ mycli --verbose input.txt
/// let args = getArgv();
/// args;  // ["--verbose", "input.txt"]
/// ```
@platform(.linux)
public func getArgv() -> Array[String] {
    var result = Array[String]();

    let path = "/proc/self/cmdline".toCString();
    let fd = std.io.libc.open(path.raw, O_RDONLY(), 0);
    path.free();
    if fd < 0 {
        return result
    }

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
                let s = String(fromUtf8: buf.asSlice()(argStart..<pos)) ?? String();
                result.append(s);
            }
            argIndex = argIndex + 1;
            argStart = pos + 1
        }
        pos = pos + 1
    }

    result
}
