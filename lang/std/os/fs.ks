// Filesystem operations
//
// Provides high-level functions for working with files and directories.

module std.os.fs

import std.num.(Int64, Int32, Int16)
import std.memory.(Pointer, RawPointer)
import std.num.(UInt8)
import std.text.(String)
import std.core.(Bool)
import std.collections.(Array)
import std.ffi.(CString)
import std.ffi.(malloc, free, memset)
import std.result.(Result)
import std.result.(Optional)
import std.io.error.(IoError)

// ============================================================================
// RAW FFI BINDINGS
// ============================================================================

@extern(.C, mangleName: "mkdir")
func libc_mkdir(path: RawPointer, mode: Int32) -> Int32

@extern(.C, mangleName: "rmdir")
func libc_rmdir(path: RawPointer) -> Int32

@extern(.C, mangleName: "unlink")
func libc_unlink(path: RawPointer) -> Int32

@extern(.C, mangleName: "rename")
func libc_rename(old: RawPointer, new: RawPointer) -> Int32

@extern(.C, mangleName: "symlink")
func libc_symlink(target: RawPointer, linkpath: RawPointer) -> Int32

@extern(.C, mangleName: "readlink")
func libc_readlink(path: RawPointer, buf: RawPointer, bufsize: Int64) -> Int64

@extern(.C, mangleName: "chmod")
func libc_chmod(path: RawPointer, mode: Int32) -> Int32

@extern(.C, mangleName: "access")
func libc_access(path: RawPointer, mode: Int32) -> Int32

@extern(.C, mangleName: "stat")
func libc_stat(path: RawPointer, buf: RawPointer) -> Int32

@extern(.C, mangleName: "opendir")
func libc_opendir(path: RawPointer) -> RawPointer

@extern(.C, mangleName: "readdir")
func libc_readdir(dirp: RawPointer) -> RawPointer

@extern(.C, mangleName: "closedir")
func libc_closedir(dirp: RawPointer) -> Int32

@extern(.C, mangleName: "getcwd")
func libc_getcwd(buf: RawPointer, size: Int64) -> RawPointer

// errno access
@platform(.darwin)
@extern(.C, mangleName: "__error")
func __errno_ptr() -> Pointer[Int32]

@platform(.linux)
@extern(.C, mangleName: "__errno_location")
func __errno_ptr() -> Pointer[Int32]

// ============================================================================
// CONSTANTS
// ============================================================================

/// `access(2)` mode flag — file existence check, no permission bits.
func F_OK() -> Int32 { 0 }

/// Conservative upper bound on `sizeof(struct stat)` across darwin/linux; used to allocate the buffer for `stat(2)`.
func STAT_BUF_SIZE() -> Int64 { 144 }

/// Byte offset of `st_mode` within `struct stat` on darwin (after `st_dev`).
@platform(.darwin)
func ST_MODE_OFFSET() -> Int64 { 4 }

/// Byte offset of `st_mode` within `struct stat` on linux (`st_dev` + `st_ino` + padding).
@platform(.linux)
func ST_MODE_OFFSET() -> Int64 { 24 }

/// `S_IFMT` — bitmask isolating the file-type bits of `st_mode`.
func S_IFMT() -> Int32 { 61440 }

/// `S_IFDIR` — file-type bits for a directory.
func S_IFDIR() -> Int32 { 16384 }

/// `S_IFREG` — file-type bits for a regular file.
func S_IFREG() -> Int32 { 32768 }

/// Byte offset of `d_name` within `struct dirent` on darwin.
@platform(.darwin)
func DIRENT_NAME_OFFSET() -> Int64 { 21 }

/// Byte offset of `d_name` within `struct dirent` on linux.
@platform(.linux)
func DIRENT_NAME_OFFSET() -> Int64 { 19 }

/// Default mode for new directories — `0o755` (`rwxr-xr-x`).
func MODE_DIR_DEFAULT() -> Int32 { 493 }

/// Reads the current value of `errno`. Platform-specific access via `__error` (darwin) or `__errno_location` (linux).
func fsErrno() -> Int32 {
    __errno_ptr().read()
}

/// Snapshots `errno` into a typed `IoError`. Call this immediately after a failing libc call before any other syscall could perturb `errno`.
func lastError() -> IoError {
    IoError(code: fsErrno())
}

// ============================================================================
// FILE EXISTENCE AND TYPE CHECKS
// ============================================================================

/// Returns true if any filesystem entry exists at `path`.
///
/// Wraps `access(path, F_OK)`. Does not distinguish files from
/// directories or symlinks (a dangling symlink reports as
/// nonexistent because `access` follows symlinks). For the type,
/// follow up with `isFile` / `isDirectory`.
///
/// # Examples
///
/// ```
/// if fileExists(path: "/tmp/foo") {
///     // ...
/// }
/// ```
public func fileExists(path: String) -> Bool {
    let cpath = path.toCString();
    let result = libc_access(cpath.raw.asRaw(), F_OK());
    cpath.free();
    result == 0
}

/// Returns true if `path` exists and is a directory.
///
/// Resolves symlinks (uses `stat`, not `lstat`). Returns `false` for
/// nonexistent paths or any non-directory file type.
///
/// # Examples
///
/// ```
/// isDirectory(path: "/tmp");      // true
/// isDirectory(path: "/etc/hosts"); // false
/// ```
public func isDirectory(path: String) -> Bool {
    let mode = statMode(path);
    match mode {
        .Some(m) => m.bitwiseAnd(S_IFMT()) == S_IFDIR(),
        .None => false
    }
}

/// Returns true if `path` exists and is a regular file.
///
/// Resolves symlinks. Returns `false` for directories, sockets,
/// FIFOs, devices, and nonexistent paths.
public func isFile(path: String) -> Bool {
    let mode = statMode(path);
    match mode {
        .Some(m) => m.bitwiseAnd(S_IFMT()) == S_IFREG(),
        .None => false
    }
}

/// Reads `st_mode` for `path` via `stat(2)`.
///
/// Returns `None` on any error (path missing, permission denied,
/// etc.). Allocates a per-call `STAT_BUF_SIZE`-byte scratch buffer;
/// the buffer is freed before return.
///
/// # Safety
///
/// Reads `sizeof(int16)` bytes at `ST_MODE_OFFSET` into the buffer
/// and zero-extends to `Int32`. Relies on the layout constants above
/// matching the host's `struct stat`.
func statMode(path: String) -> Optional[Int32] {
    let cpath = path.toCString();
    let buf = malloc(STAT_BUF_SIZE());
    let _ = memset(buf, 0, STAT_BUF_SIZE());

    let result = libc_stat(cpath.raw.asRaw(), buf);
    cpath.free();

    if result != 0 {
        free(buf);
        return .None
    }

    let modePtr = buf.offset(by: ST_MODE_OFFSET());
    let modeRaw = modePtr.cast[Int16]().read();
    let mode = Int32(from: modeRaw);

    free(buf);
    .Some(mode)
}

// ============================================================================
// DIRECTORY OPERATIONS
// ============================================================================

/// Creates a single directory at `path` with mode `0o755`.
///
/// Wraps `mkdir(2)`. Fails if `path` already exists or any parent
/// component is missing — use `mkdirAll` to create intermediate
/// directories.
///
/// # Errors
///
/// Returns `Err(IoError)` on any libc failure; `errno` is captured and
/// surfaced via the error's `kind`. Common cases: `EEXIST` (path exists),
/// `ENOENT` (missing parent), `EACCES` (permission denied).
///
/// # Examples
///
/// ```
/// match mkdir(path: "/tmp/foo") {
///     .Ok(_)  => print("created"),
///     .Err(e) => print(e.message)
/// }
/// ```
public func mkdir(path: String) -> Result[(), IoError] {
    let cpath = path.toCString();
    let result = libc_mkdir(cpath.raw.asRaw(), MODE_DIR_DEFAULT());
    cpath.free();

    if result != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Creates `path` and any missing parent directories.
///
/// Walks back from the deepest non-existent component, recursing on
/// the parent first. If `path` already exists and is a directory, the
/// call is a no-op success; if it exists and is **not** a directory,
/// returns `Err(IoError(kind: .AlreadyExists))` (`EEXIST`) without disturbing the file.
/// Each created intermediate uses the default mode `0o755`.
///
/// # Errors
///
/// Forwards any `mkdir` failure verbatim. Specific to this function:
/// `Err(IoError(kind: .AlreadyExists))` when `path` exists as a non-directory.
///
/// # Examples
///
/// ```
/// mkdirAll(path: "/tmp/foo/bar/baz");  // creates all three levels
/// ```
public func mkdirAll(path: String) -> Result[(), IoError] {
    if fileExists(path) {
        if isDirectory(path) {
            return .Ok(())
        }
        return .Err(IoError(kind: .AlreadyExists))
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

/// Removes an empty directory at `path`.
///
/// Wraps `rmdir(2)`. Fails with `ENOTEMPTY` if the directory still
/// has entries — list and remove its contents first if you need a
/// recursive remove.
///
/// # Errors
///
/// Returns `Err(IoError)` on any libc failure; `errno` is captured.
public func removeDir(path: String) -> Result[(), IoError] {
    let cpath = path.toCString();
    let result = libc_rmdir(cpath.raw.asRaw());
    cpath.free();

    if result != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Returns the names of the entries inside `path`, excluding `.` and `..`.
///
/// Wraps `opendir`/`readdir`/`closedir`. The returned names are
/// relative to `path`; join with `path` yourself if you need full
/// paths. On failure to open the directory (missing path, permission
/// denied, etc.), returns an empty array — the function does not
/// distinguish "empty directory" from "open failed".
///
/// # Examples
///
/// ```
/// for entry in listDir(path: "/tmp") {
///     print(entry);
/// }
/// ```
public func listDir(path: String) -> Array[String] {
    var result = Array[String]();
    let cpath = path.toCString();
    let dirp = libc_opendir(cpath.raw.asRaw());
    cpath.free();

    if dirp.isNull {
        return result
    }

    while true {
        let entry = libc_readdir(dirp);
        if entry.isNull {
            break
        }

        let namePtr = entry.offset(by: DIRENT_NAME_OFFSET());
        let cstr = CString(raw: namePtr.cast[UInt8]());
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

/// Deletes the file at `path`.
///
/// Wraps `unlink(2)`. Does not work on directories — use `removeDir`
/// for those. If `path` is the last link to an open file, the file's
/// blocks remain allocated until every descriptor is closed.
///
/// # Errors
///
/// Returns `Err(IoError)` on any libc failure; `errno` is captured.
public func remove(path: String) -> Result[(), IoError] {
    let cpath = path.toCString();
    let result = libc_unlink(cpath.raw.asRaw());
    cpath.free();

    if result != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Renames or moves `from` to `to`.
///
/// Wraps `rename(2)`. Atomic within a single filesystem; cross-
/// filesystem moves return `EXDEV` and require a copy + delete
/// instead. If `to` exists, it is replaced (subject to type-match
/// rules — file replaces file, directory replaces empty directory).
///
/// # Errors
///
/// Returns `Err(IoError)` on any libc failure; `errno` is captured.
public func rename(from: String, to: String) -> Result[(), IoError] {
    let cfrom = from.toCString();
    let cto = to.toCString();
    let result = libc_rename(cfrom.raw.asRaw(), cto.raw.asRaw());
    cfrom.free();
    cto.free();

    if result != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

// ============================================================================
// SYMBOLIC LINKS
// ============================================================================

/// Creates a symbolic link at `path` pointing to `target`.
///
/// Wraps `symlink(2)`. The target is stored verbatim — it is not
/// resolved or validated, so dangling links are allowed and relative
/// targets resolve relative to the directory containing the link.
///
/// # Errors
///
/// Returns `Err(IoError)` on any libc failure; `errno` is captured.
public func symlink(target: String, path: String) -> Result[(), IoError] {
    let ctarget = target.toCString();
    let cpath = path.toCString();
    let result = libc_symlink(ctarget.raw.asRaw(), cpath.raw.asRaw());
    ctarget.free();
    cpath.free();

    if result != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

/// Returns the target stored in the symlink at `path`.
///
/// Wraps `readlink(2)` with a 1 KiB buffer. `readlink` does not null-
/// terminate, so this function copies exactly the returned byte
/// count into the result string. Targets longer than 1 KiB are
/// silently truncated — the syscall returns a partial result rather
/// than failing.
///
/// # Errors
///
/// Returns `Err(IoError)` if `path` is not a symlink (`EINVAL`),
/// missing (`ENOENT`), or any other libc failure.
public func readlink(path: String) -> Result[String, IoError] {
    let cpath = path.toCString();
    let bufsize: Int64 = 1024;
    let buf = malloc(bufsize);

    let bytesRead = libc_readlink(cpath.raw.asRaw(), buf, bufsize);
    cpath.free();

    if bytesRead < 0 {
        free(buf);
        return .Err(lastError())
    }

    var result = String();
    var i: Int64 = 0;
    while i < bytesRead {
        let bytePtr = buf.offset(by: i).cast[UInt8]();
        let ch = bytePtr.read();
        result.appendByte(ch);
        i = i + 1
    }

    free(buf);
    .Ok(result)
}

// ============================================================================
// PERMISSIONS
// ============================================================================

/// Changes the mode bits of `path` to `mode`.
///
/// Wraps `chmod(2)`. `mode` is the raw POSIX mode bits (e.g.
/// `0o755`); pass it as `Int32`. Resolves symlinks (use `lchmod`
/// equivalent if you need to change a link itself — not currently
/// exposed).
///
/// # Errors
///
/// Returns `Err(IoError)` on any libc failure; `errno` is captured.
///
/// # Examples
///
/// ```
/// chmod(path: "/tmp/script.sh", mode: Int32(intLiteral: 0o755));
/// ```
public func chmod(path: String, mode: Int32) -> Result[(), IoError] {
    let cpath = path.toCString();
    let result = libc_chmod(cpath.raw.asRaw(), mode);
    cpath.free();

    if result != 0 {
        return .Err(lastError())
    }
    .Ok(())
}

// ============================================================================
// WORKING DIRECTORY
// ============================================================================

/// Returns the calling process's current working directory.
///
/// Wraps `getcwd(2)` with a 1 KiB buffer. Returns the empty string if
/// the cwd has been deleted, is longer than 1 KiB, or any other
/// `getcwd` failure occurs — the function does not surface the
/// error code.
///
/// # Examples
///
/// ```
/// let here = getcwd();
/// ```
public func getcwd() -> String {
    let size: Int64 = 1024;
    let buf = malloc(size);
    let result = libc_getcwd(buf, size);

    if result.isNull {
        free(buf);
        return String()
    }

    let cstr = CString(raw: buf.cast[UInt8]());
    let s = String(from: cstr);
    free(buf);
    s
}

// ============================================================================
// HELPERS
// ============================================================================

/// Returns the byte offset of the last `/` in `s`, or `-1` if there is none.
///
/// Used by `mkdirAll` to find the parent directory boundary. Plain
/// byte scan — works correctly for UTF-8 because `/` is ASCII and
/// cannot appear inside a multi-byte sequence.
func findLastSlash(s: String) -> Int64 {
    let len = s.byteCount;
    var i: Int64 = len - 1;
    while i >= 0 {
        if s.bytes(unchecked: i) == UInt8(intLiteral: 47) {
            return i
        }
        i = i - 1
    }
    return -1
}
