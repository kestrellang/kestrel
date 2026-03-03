// OS-level FFI bindings for CLI tools
//
// Provides access to command-line arguments, filesystem operations,
// environment variables, and process spawning on macOS.

module clutch.os

// ============================================================================
// RAW FFI BINDINGS
// ============================================================================

// Command-line arguments (macOS)
@extern(.C, mangleName: "_NSGetArgc")
func nsGetArgc() -> lang.ptr[lang.i32]

@extern(.C, mangleName: "_NSGetArgv")
func nsGetArgv() -> lang.ptr[lang.ptr[lang.ptr[lang.i8]]]

// Process spawning
@extern(.C, mangleName: "system")
func libc_system(cmd: lang.ptr[lang.i8]) -> lang.i32

// Working directory
@extern(.C, mangleName: "getcwd")
func libc_getcwd(buf: lang.ptr[lang.i8], size: lang.i64) -> lang.ptr[lang.i8]

// Environment
@extern(.C, mangleName: "getenv")
func libc_getenv(name: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

// File existence
@extern(.C, mangleName: "access")
func libc_access(path: lang.ptr[lang.i8], mode: lang.i32) -> lang.i32

// Stat for directory check
@extern(.C, mangleName: "stat")
func libc_stat(path: lang.ptr[lang.i8], buf: lang.ptr[lang.i8]) -> lang.i32

// Process output capture
@extern(.C, mangleName: "popen")
func libc_popen(cmd: lang.ptr[lang.i8], mode: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "pclose")
func libc_pclose(stream: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "fgets")
func libc_fgets(buf: lang.ptr[lang.i8], size: lang.i32, stream: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

// Directory listing
@extern(.C, mangleName: "opendir")
func libc_opendir(path: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "readdir")
func libc_readdir(dirp: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "closedir")
func libc_closedir(dirp: lang.ptr[lang.i8]) -> lang.i32

// ============================================================================
// CONSTANTS
// ============================================================================

// access() mode: check file existence
func F_OK() -> Int32 { 0 }

// macOS stat struct: st_mode is at byte offset 4 (after st_dev:4)
// st_mode is UInt16. S_IFDIR = 0o040000 = 16384, S_IFMT = 0o170000 = 61440
func STAT_BUF_SIZE() -> Int64 { 144 } // macOS struct stat size
func ST_MODE_OFFSET() -> Int64 { 4 }
func S_IFMT() -> Int32 { 61440 }
func S_IFDIR() -> Int32 { 16384 }

// macOS dirent: d_name is at byte offset 21
// d_ino(8) + d_seekoff(8) + d_reclen(2) + d_namlen(2) + d_type(1) = 21
func DIRENT_NAME_OFFSET() -> Int64 { 21 }

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

/// Returns the current working directory.
public func getcwd() -> String {
    let size: Int64 = 1024;
    let buf = malloc(size.raw);
    let result = libc_getcwd(buf, size.raw);

    if Bool(boolLiteral: lang.ptr_is_null(result)) {
        free(buf);
        return String()
    }

    let cstr = CString(raw: Pointer(raw: lang.cast_ptr[UInt8](buf)));
    let s = String(from: cstr);
    free(buf);
    s
}

/// Gets an environment variable by name. Returns None if not set.
public func getenv(name name: String) -> Optional[String] {
    let cname = name.toCString();
    let result = libc_getenv(lang.cast_ptr[lang.i8](cname.raw.raw));
    cname.free();

    if Bool(boolLiteral: lang.ptr_is_null(result)) {
        return .None
    }

    let cstr = CString(raw: Pointer(raw: lang.cast_ptr[UInt8](result)));
    .Some(String(from: cstr))
    // Note: do not free result - it points to environ memory
}

/// Returns true if a file or directory exists at the given path.
public func fileExists(path path: String) -> Bool {
    let cpath = path.toCString();
    let result = libc_access(lang.cast_ptr[lang.i8](cpath.raw.raw), F_OK().raw);
    cpath.free();
    Int32(raw: result) == 0
}

/// Returns true if the path is a directory.
public func isDirectory(path path: String) -> Bool {
    let cpath = path.toCString();
    let buf = malloc(STAT_BUF_SIZE().raw);

    // Zero out the buffer
    let _ = std.ffi.memset(buf, 0, STAT_BUF_SIZE().raw);

    let result = libc_stat(lang.cast_ptr[lang.i8](cpath.raw.raw), buf);
    cpath.free();

    if Int32(raw: result) != 0 {
        free(buf);
        return false
    }

    // Read st_mode (UInt16 at offset 4 on macOS)
    let modePtr = lang.ptr_offset(buf, ST_MODE_OFFSET().raw);
    let modeRaw = lang.ptr_read(lang.cast_ptr[lang.i16](modePtr));
    let mode = Int32(raw: lang.cast_i16_i32(modeRaw));

    free(buf);

    // Check S_ISDIR: (mode & S_IFMT) == S_IFDIR
    let masked = Int32(raw: lang.i32_and(mode.raw, S_IFMT().raw));
    masked == S_IFDIR()
}

/// Lists all entries in a directory (excluding "." and "..").
/// Returns an empty array if the directory cannot be opened.
public func listDir(path path: String) -> Array[String] {
    var result = Array[String]();
    let cpath = path.toCString();
    let dirp = libc_opendir(lang.cast_ptr[lang.i8](cpath.raw.raw));
    cpath.free();

    if Bool(boolLiteral: lang.ptr_is_null(dirp)) {
        return result
    }

    while true {
        let entry = libc_readdir(dirp);
        if Bool(boolLiteral: lang.ptr_is_null(entry)) {
            break
        }

        // Extract d_name from dirent struct (offset 21 on macOS)
        let namePtr = lang.ptr_offset(entry, DIRENT_NAME_OFFSET().raw);
        let cstr = CString(raw: Pointer(raw: lang.cast_ptr[UInt8](namePtr)));
        let name = String(from: cstr);
        // Note: do not free cstr - it points into the dirent struct

        // Skip . and ..
        if name.equals(".") or name.equals("..") {
            // continue - skip these entries
        } else {
            result.append(name)
        }
    }

    let _ = libc_closedir(dirp);
    result
}

/// Runs a shell command and returns the exit code.
/// The command's stdout/stderr go directly to the terminal.
public func spawn(command command: String) -> Int32 {
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
public func captureOutput(command command: String) -> String {
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

    // Trim trailing whitespace (newlines)
    trimEnd(output)
}

/// Removes trailing whitespace characters from a string.
func trimEnd(s: String) -> String {
    var end = s.byteCount;
    while end > 0 {
        let b = s.byteAtUnchecked(end - 1);
        // space=32, tab=9, newline=10, carriage return=13
        if b == UInt8(intLiteral: 32) or b == UInt8(intLiteral: 9) or b == UInt8(intLiteral: 10) or b == UInt8(intLiteral: 13) {
            end = end - 1
        } else {
            break
        }
    }
    s.substringBytes(from: 0, to: end)
}
