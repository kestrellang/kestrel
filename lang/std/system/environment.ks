// The process environment: variables and directories.

module std.system

import std.numeric.(Int32, Int64, UInt8)
import std.memory.(RawPointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.result.(Optional, Result)
import std.io.error.(IoError)
import std.ffi.(malloc, free)

/// Environment variables and the process's working, home and temporary
/// directories.
///
/// A caseless enum used as a namespace: every member is `static`.
///
/// # Examples
///
/// ```
/// if let .Some(home) = Environment.variable("HOME") {
///     print(home)
/// }
/// let cwd = try Environment.currentDirectory();
/// ```
public enum Environment {
    /// The value of variable `name`, or `None` if it is unset or its value
    /// is not valid UTF-8. Use `variableBytes` for values that may not be text.
    ///
    /// The value is copied out, so it stays valid after later `setVariable` calls.
    public static func variable(name: String) -> Optional[String] {
        let bytes = try Environment.variableBytes(name);
        String(fromUtf8: bytes.asSlice())
    }

    /// The raw bytes of variable `name`, or `None` if it is unset.
    public static func variableBytes(name: String) -> Optional[Array[UInt8]] {
        let cname = name.toCString();
        let raw = native_getenv(cname.raw.asRaw());
        cname.free();
        if raw.isNull {
            return .None
        }
        .Some(bytesFromNative(raw))
    }

    /// True if variable `name` is set, even to the empty string.
    public static func hasVariable(name: String) -> Bool {
        let cname = name.toCString();
        let raw = native_getenv(cname.raw.asRaw());
        cname.free();
        not raw.isNull
    }

    /// A snapshot of every variable as `(name, value)` pairs, in the order
    /// the C runtime stores them. Entries that aren't valid UTF-8 are
    /// skipped.
    public static func variables() -> Array[(String, String)] {
        var result = Array[(String, String)]();
        let environ = native_environ();
        if environ.isNull {
            return result
        }
        var i: Int64 = 0;
        while true {
            let entry = nativePointerAt(environ, i);
            if entry.isNull {
                break
            }
            i = i + 1;
            if let .Some(pair) = splitEnvironmentEntry(bytesFromNative(entry)) {
                result.append(pair)
            }
        }
        result
    }

    /// Sets variable `name` to `value`, replacing any old value.
    ///
    /// # Safety
    ///
    /// Not thread-safe: C code on another thread that reads the environment
    /// (`getenv`, DNS lookups, locale setup) can crash while this runs. Call
    /// it before starting threads. To give a child process a different
    /// environment, use `Command.environment(_:to:)` instead. This will
    /// become an `unsafe` function once the keyword exists.
    ///
    /// # Errors
    ///
    /// `InvalidInput` if `name` is empty or contains `=`.
    public static func setVariable(name: String, to value: String) -> Result[(), IoError] {
        let cname = name.toCString();
        let cvalue = value.toCString();
        let status = native_setenv(cname.raw.asRaw(), cvalue.raw.asRaw(), 1);
        cname.free();
        cvalue.free();
        if status != 0 {
            return .Err(IoError.last())
        }
        .Ok(())
    }

    /// Removes variable `name`. Removing an unset variable succeeds.
    ///
    /// # Safety
    ///
    /// Not thread-safe, for the same reason as `setVariable`.
    public static func removeVariable(name: String) -> Result[(), IoError] {
        let cname = name.toCString();
        let status = native_unsetenv(cname.raw.asRaw());
        cname.free();
        if status != 0 {
            return .Err(IoError.last())
        }
        .Ok(())
    }

    /// The process's current working directory, as an absolute path.
    ///
    /// # Errors
    ///
    /// `NotFound` if the directory was deleted; `PermissionDenied` if a
    /// parent can't be read.
    public static func currentDirectory() -> Result[String, IoError] {
        var size: Int64 = 512;
        while true {
            let buf = malloc(size);
            let result = native_getcwd(buf, size);
            if not result.isNull {
                let path = stringFromNative(buf) ?? String();
                free(buf);
                return .Ok(path)
            }
            let error = IoError.last();
            free(buf);
            // ERANGE: the buffer was too small, so grow it and retry.
            if error.errno() != rangeErrorCode() or size >= 1048576 {
                return .Err(error)
            }
            size = size * 2
        }
        .Ok(String())
    }

    /// Changes the process's working directory. Affects every thread.
    public static func setCurrentDirectory(to path: String) -> Result[(), IoError] {
        let cpath = path.toCString();
        let status = native_chdir(cpath.raw.asRaw());
        cpath.free();
        if status != 0 {
            return .Err(IoError.last())
        }
        .Ok(())
    }

    /// The current user's home directory: `$HOME` if set and non-empty,
    /// else the password database entry. `None` if neither has one.
    public static func homeDirectory() -> Optional[String] {
        if let .Some(home) = Environment.variable("HOME") {
            if not home.isEmpty {
                return .Some(home)
            }
        }
        stringFromNative(native_passwd_home())
    }

    /// The directory for temporary files: `$TMPDIR` if set and non-empty,
    /// else `/tmp`. Not checked for existence.
    public static func temporaryDirectory() -> String {
        if let .Some(dir) = Environment.variable("TMPDIR") {
            if not dir.isEmpty {
                return dir
            }
        }
        "/tmp"
    }
}

/// Splits a `NAME=value` environment entry at its first `=`. `None` if
/// there is no `=` or either side isn't UTF-8.
func splitEnvironmentEntry(entry: Array[UInt8]) -> Optional[(String, String)] {
    var i: Int64 = 0;
    // Start at 1: Windows-style `=C:` names begin with `=`, never split there.
    while i < entry.count {
        if i > 0 and entry(i) == 61 {
            let name = try String(fromUtf8: entry(0..<i));
            let value = try String(fromUtf8: entry((i + 1)..<entry.count));
            return .Some((name, value))
        }
        i = i + 1
    }
    .None
}

/// `ERANGE` is 34 on both Linux and Darwin.
func rangeErrorCode() -> Int32 { 34 }
