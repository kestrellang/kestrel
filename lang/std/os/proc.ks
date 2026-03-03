// Process spawning and control

module std.os.proc

import std.num.(Int64, Int32)
import std.num.(UInt8)
import std.memory.(Pointer)
import std.text.(String)
import std.core.(Bool)
import std.ffi.(CString)
import std.ffi.(malloc, free)

// ============================================================================
// RAW FFI BINDINGS
// ============================================================================

@extern(.C, mangleName: "system")
func libc_system(cmd: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "popen")
func libc_popen(cmd: lang.ptr[lang.i8], mode: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "pclose")
func libc_pclose(stream: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "fgets")
func libc_fgets(buf: lang.ptr[lang.i8], size: lang.i32, stream: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "exit")
func libc_exit(code: lang.i32)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Runs a shell command and returns the exit code.
/// The command's stdout/stderr go directly to the terminal.
public func spawn(command: String) -> Int32 {
    let ccmd = command.toCString();
    let rawStatus = libc_system(lang.cast_ptr[lang.i8](ccmd.raw.raw));
    ccmd.free();
    // system() returns the exit status in the upper bits on POSIX
    // Shift right by 8 to get the actual exit code
    let status = Int32(raw: rawStatus);
    status >> 8
}

/// Runs a shell command and captures its stdout as a string.
/// Returns the trimmed output, or an empty string if the command fails.
public func captureOutput(command: String) -> String {
    let ccmd = command.toCString();
    let modeStr = "r".toCString();
    let stream = libc_popen(
        lang.cast_ptr[lang.i8](ccmd.raw.raw),
        lang.cast_ptr[lang.i8](modeStr.raw.raw)
    );
    ccmd.free();
    modeStr.free();

    if Bool(boolLiteral: lang.ptr_is_null(stream)) {
        return String()
    }

    var output = String();
    let bufSize: Int32 = 1024;
    let buf = malloc(Int64(from: bufSize).raw);

    while true {
        let line = libc_fgets(buf, bufSize.raw, stream);
        if Bool(boolLiteral: lang.ptr_is_null(line)) {
            break
        }
        let cstr = CString(raw: Pointer(raw: lang.cast_ptr[UInt8](buf)));
        output = output + String(from: cstr)
    }

    free(buf);
    let _ = libc_pclose(stream);

    trimEnd(output)
}

/// Terminates the process with the given exit code.
public func exit(code: Int32) {
    libc_exit(code.raw)
}

/// Removes trailing whitespace characters from a string.
func trimEnd(s: String) -> String {
    var end = s.byteCount;
    while end > 0 {
        let b = s.byteAtUnchecked(end - 1);
        if b == UInt8(intLiteral: 32) or b == UInt8(intLiteral: 9) or b == UInt8(intLiteral: 10) or b == UInt8(intLiteral: 13) {
            end = end - 1
        } else {
            break
        }
    }
    s.substringBytes(from: 0, to: end)
}
