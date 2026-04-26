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

/// Runs `command` through the system shell and returns its exit code.
///
/// Wraps `libc::system`, which on POSIX runs `/bin/sh -c <command>`
/// and returns a packed status word; this function shifts off the
/// signal/coredump bits and returns just the exit code (0–255 in
/// normal cases). The child's stdout and stderr are inherited from
/// the parent process — they go straight to the terminal. For
/// captured output, use `captureOutput`.
///
/// # Examples
///
/// ```
/// let code = spawn(command: "ls -la");
/// if code != Int32(intLiteral: 0) {
///     print("ls failed");
/// }
/// ```
public func spawn(command: String) -> Int32 {
    let ccmd = command.toCString();
    let rawStatus = libc_system(lang.cast_ptr[_, lang.i8](ccmd.raw.raw));
    ccmd.free();
    // system() returns the exit status in the upper bits on POSIX
    // Shift right by 8 to get the actual exit code
    let status = Int32(raw: rawStatus);
    status >> 8
}

/// Runs `command` through the system shell and returns its captured stdout.
///
/// Reads from `popen(command, "r")` 1 KiB at a time until EOF, then
/// trims a single run of trailing ASCII whitespace (space, tab, LF,
/// CR) so callers don't have to chomp the newline themselves. Stderr
/// is **not** captured — it goes to the parent's stderr. Returns the
/// empty string if `popen` fails.
///
/// # Examples
///
/// ```
/// let branch = captureOutput(command: "git rev-parse --abbrev-ref HEAD");
/// // "main"
/// ```
public func captureOutput(command: String) -> String {
    let ccmd = command.toCString();
    let modeStr = "r".toCString();
    let stream = libc_popen(
        lang.cast_ptr[_, lang.i8](ccmd.raw.raw),
        lang.cast_ptr[_, lang.i8](modeStr.raw.raw)
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
        let cstr = CString(raw: Pointer(raw: lang.cast_ptr[_, UInt8](buf)));
        output = output + String(from: cstr)
    }

    free(buf);
    let _ = libc_pclose(stream);

    trimEnd(output)
}

/// Terminates the calling process immediately with the given exit code.
///
/// Wraps `libc::exit`. Runs `atexit` handlers and flushes stdio
/// buffers; does **not** unwind Kestrel's stack or run deinits on
/// values still in scope. Conventionally `0` means success and any
/// non-zero value means failure; a few codes have specific meanings
/// (`2` is shells' "misuse of builtins", `126`/`127` are `exec`
/// errors, `>128` typically encodes a fatal signal).
///
/// # Examples
///
/// ```
/// exit(code: Int32(intLiteral: 0));   // success — does not return
/// ```
public func exit(code: Int32) {
    libc_exit(code.raw)
}

/// Strips a trailing run of ASCII whitespace (space, tab, LF, CR) from `s`.
///
/// Used by `captureOutput` to chomp the trailing newline before
/// returning. Local helper rather than a `String.trimmedEnd`
/// dependency to keep `std.os.proc` light.
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
