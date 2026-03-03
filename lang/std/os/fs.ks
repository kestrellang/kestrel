// Filesystem operations
//
// Provides high-level functions for working with files and directories.

module std.os.fs

import std.num.(Int64, Int32)
import std.memory.(Pointer)
import std.num.(UInt8)
import std.text.(String)
import std.core.(Bool)
import std.collections.(Array)
import std.ffi.(CString)
import std.ffi.(malloc, free, memset)
import std.result.(Result)
import std.result.(Optional)
import std.io.error.(Error)

// ============================================================================
// RAW FFI BINDINGS
// ============================================================================

@extern(.C, mangleName: "mkdir")
func libc_mkdir(path: lang.ptr[lang.i8], mode: lang.i32) -> lang.i32

@extern(.C, mangleName: "rmdir")
func libc_rmdir(path: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "unlink")
func libc_unlink(path: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "rename")
func libc_rename(old: lang.ptr[lang.i8], new: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "symlink")
func libc_symlink(target: lang.ptr[lang.i8], linkpath: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "readlink")
func libc_readlink(path: lang.ptr[lang.i8], buf: lang.ptr[lang.i8], bufsize: lang.i64) -> lang.i64

@extern(.C, mangleName: "chmod")
func libc_chmod(path: lang.ptr[lang.i8], mode: lang.i32) -> lang.i32

@extern(.C, mangleName: "access")
func libc_access(path: lang.ptr[lang.i8], mode: lang.i32) -> lang.i32

@extern(.C, mangleName: "stat")
func libc_stat(path: lang.ptr[lang.i8], buf: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "opendir")
func libc_opendir(path: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "readdir")
func libc_readdir(dirp: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "closedir")
func libc_closedir(dirp: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "getcwd")
func libc_getcwd(buf: lang.ptr[lang.i8], size: lang.i64) -> lang.ptr[lang.i8]

// __errno_ptr() is in fs.darwin.ks / fs.linux.ks

// ============================================================================
// CONSTANTS
// ============================================================================

func F_OK() -> Int32 { 0 }
func STAT_BUF_SIZE() -> Int64 { 144 }
// ST_MODE_OFFSET is in fs.darwin.ks / fs.linux.ks
func S_IFMT() -> Int32 { 61440 }
func S_IFDIR() -> Int32 { 16384 }
func S_IFREG() -> Int32 { 32768 }
// DIRENT_NAME_OFFSET is in fs.darwin.ks / fs.linux.ks
func MODE_DIR_DEFAULT() -> Int32 { 493 }

func fsErrno() -> Int32 {
    let ptr = __errno_ptr();
    Int32(raw: lang.ptr_read(ptr))
}

func lastError() -> Error {
    Error(fsErrno())
}

// ============================================================================
// FILE EXISTENCE AND TYPE CHECKS
// ============================================================================

/// Returns true if a file or directory exists at the given path.
public func fileExists(path: String) -> Bool {
    let cpath = path.toCString();
    let result = libc_access(lang.cast_ptr[lang.i8](cpath.raw.raw), F_OK().raw);
    cpath.free();
    Int32(raw: result) == 0
}

/// Returns true if the path is a directory.
public func isDirectory(path: String) -> Bool {
    let mode = statMode(path);
    match mode {
        .Some(m) => {
            let masked = Int32(raw: lang.i32_and(m.raw, S_IFMT().raw));
            masked == S_IFDIR()
        },
        .None => false
    }
}

/// Returns true if the path is a regular file.
public func isFile(path: String) -> Bool {
    let mode = statMode(path);
    match mode {
        .Some(m) => {
            let masked = Int32(raw: lang.i32_and(m.raw, S_IFMT().raw));
            masked == S_IFREG()
        },
        .None => false
    }
}

func statMode(path: String) -> Optional[Int32] {
    let cpath = path.toCString();
    let buf = malloc(STAT_BUF_SIZE().raw);
    let _ = memset(buf, 0, STAT_BUF_SIZE().raw);

    let result = libc_stat(lang.cast_ptr[lang.i8](cpath.raw.raw), buf);
    cpath.free();

    if Int32(raw: result) != 0 {
        free(buf);
        return .None
    }

    let modePtr = lang.ptr_offset(buf, ST_MODE_OFFSET().raw);
    let modeRaw = lang.ptr_read(lang.cast_ptr[lang.i16](modePtr));
    let mode = Int32(raw: lang.cast_i16_i32(modeRaw));

    free(buf);
    .Some(mode)
}

// ============================================================================
// DIRECTORY OPERATIONS
// ============================================================================

/// Creates a directory at the given path.
public func mkdir(path: String) -> Result[(), Error] {
    let cpath = path.toCString();
    let result = libc_mkdir(lang.cast_ptr[lang.i8](cpath.raw.raw), MODE_DIR_DEFAULT().raw);
    cpath.free();

    if Int32(raw: result) != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Creates a directory and all parent directories as needed.
public func mkdirAll(path: String) -> Result[(), Error] {
    if fileExists(path) {
        if isDirectory(path) {
            return .Ok(())
        }
        return .Err(Error(17))
    }

    let lastSlash = findLastSlash(path);
    if lastSlash > 0 {
        let parent = path.substringBytes(from: 0, to: lastSlash);
        let parentResult = mkdirAll(parent);
        match parentResult {
            .Err(e) => { return .Err(e) },
            .Ok(_) => {}
        }
    }

    mkdir(path)
}

/// Removes an empty directory.
public func removeDir(path: String) -> Result[(), Error] {
    let cpath = path.toCString();
    let result = libc_rmdir(lang.cast_ptr[lang.i8](cpath.raw.raw));
    cpath.free();

    if Int32(raw: result) != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Lists all entries in a directory (excluding "." and "..").
/// Returns an empty array if the directory cannot be opened.
public func listDir(path: String) -> Array[String] {
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

        let namePtr = lang.ptr_offset(entry, DIRENT_NAME_OFFSET().raw);
        let cstr = CString(raw: Pointer(raw: lang.cast_ptr[UInt8](namePtr)));
        let name = String(from: cstr);

        if name.equals(".") or name.equals("..") {
            // skip
        } else {
            result.append(name)
        }
    }

    let _ = libc_closedir(dirp);
    result
}

// ============================================================================
// FILE OPERATIONS
// ============================================================================

/// Deletes a file.
public func remove(path: String) -> Result[(), Error] {
    let cpath = path.toCString();
    let result = libc_unlink(lang.cast_ptr[lang.i8](cpath.raw.raw));
    cpath.free();

    if Int32(raw: result) != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Renames or moves a file or directory.
public func rename(from: String, to: String) -> Result[(), Error] {
    let cfrom = from.toCString();
    let cto = to.toCString();
    let result = libc_rename(lang.cast_ptr[lang.i8](cfrom.raw.raw), lang.cast_ptr[lang.i8](cto.raw.raw));
    cfrom.free();
    cto.free();

    if Int32(raw: result) != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

// ============================================================================
// SYMBOLIC LINKS
// ============================================================================

/// Creates a symbolic link at linkPath pointing to target.
public func symlink(target: String, path: String) -> Result[(), Error] {
    let ctarget = target.toCString();
    let cpath = path.toCString();
    let result = libc_symlink(lang.cast_ptr[lang.i8](ctarget.raw.raw), lang.cast_ptr[lang.i8](cpath.raw.raw));
    ctarget.free();
    cpath.free();

    if Int32(raw: result) != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Reads the target of a symbolic link.
public func readlink(path: String) -> Result[String, Error] {
    let cpath = path.toCString();
    let bufsize: Int64 = 1024;
    let buf = malloc(bufsize.raw);

    let bytesRead = libc_readlink(lang.cast_ptr[lang.i8](cpath.raw.raw), buf, bufsize.raw);
    cpath.free();

    if Int64(raw: bytesRead) < 0 {
        free(buf);
        return .Err(lastError())
    }

    // Build string from bytes (readlink does not null-terminate)
    var result = String();
    var i: Int64 = 0;
    let count = Int64(raw: bytesRead);
    while i < count {
        let byte = lang.ptr_read(lang.cast_ptr[lang.i8](lang.ptr_offset(buf, i.raw)));
        let ch = UInt8(raw: lang.cast_i8_u8(byte));
        result.appendByte(ch);
        i = i + 1
    }

    free(buf);
    .Ok(result)
}

// ============================================================================
// PERMISSIONS
// ============================================================================

/// Changes the permissions of a file or directory.
public func chmod(path: String, mode: Int32) -> Result[(), Error] {
    let cpath = path.toCString();
    let result = libc_chmod(lang.cast_ptr[lang.i8](cpath.raw.raw), mode.raw);
    cpath.free();

    if Int32(raw: result) != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

// ============================================================================
// WORKING DIRECTORY
// ============================================================================

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

// ============================================================================
// HELPERS
// ============================================================================

func findLastSlash(s: String) -> Int64 {
    let len = s.byteCount;
    var i: Int64 = len - 1;
    while i >= 0 {
        if s.byteAtUnchecked(i) == UInt8(intLiteral: 47) {
            return i
        }
        i = i - 1
    }
    return -1
}
