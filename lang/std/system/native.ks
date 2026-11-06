// Native bindings and helpers shared by the std.system files.
//
// Everything here is internal. The public API in the other files never
// exposes a raw pointer or a file descriptor number. Calls that need a
// target-specific struct layout go through `system_shims.c`.

module std.system

import std.numeric.(Int32, Int64, UInt8)
import std.memory.(Pointer, RawPointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.ffi.(CString)
import std.result.(Optional)

// ============================================================================
// libc
// ============================================================================

@extern(.C, mangleName: "getenv")
internal func native_getenv(name: RawPointer) -> RawPointer

@extern(.C, mangleName: "setenv")
internal func native_setenv(name: RawPointer, value: RawPointer, overwrite: Int32) -> Int32

@extern(.C, mangleName: "unsetenv")
internal func native_unsetenv(name: RawPointer) -> Int32

@extern(.C, mangleName: "getcwd")
internal func native_getcwd(buf: RawPointer, size: Int64) -> RawPointer

@extern(.C, mangleName: "chdir")
internal func native_chdir(path: RawPointer) -> Int32

@extern(.C, mangleName: "getpid")
internal func native_getpid() -> Int32

@extern(.C, mangleName: "getppid")
internal func native_getppid() -> Int32

@extern(.C, mangleName: "gethostname")
internal func native_gethostname(buf: RawPointer, size: Int64) -> Int32

@extern(.C, mangleName: "kill")
internal func native_kill(pid: Int32, signal: Int32) -> Int32

@extern(.C, mangleName: "close")
internal func native_close(fd: Int32) -> Int32

@extern(.C, mangleName: "read")
internal func native_read(fd: Int32, buf: RawPointer, count: Int64) -> Int64

@extern(.C, mangleName: "write")
internal func native_write(fd: Int32, buf: RawPointer, count: Int64) -> Int64

@extern(.C, mangleName: "exit")
internal func native_exit(code: Int32)

@extern(.C, mangleName: "abort")
internal func native_abort()

// ============================================================================
// system_shims.c
// ============================================================================

@extern(.C, mangleName: "kestrel_argc")
internal func native_argc() -> Int32

@extern(.C, mangleName: "kestrel_argv")
internal func native_argv() -> RawPointer

@extern(.C, mangleName: "kestrel_environ")
internal func native_environ() -> RawPointer

@extern(.C, mangleName: "kestrel_spawn")
internal func native_spawn(program: RawPointer, argv: RawPointer, envp: RawPointer, cwd: RawPointer,
                  modes: RawPointer, pidOut: RawPointer, parentFds: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_wait")
internal func native_wait(pid: Int32, nohang: Int32, kind: RawPointer, value: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_poll_two")
internal func native_poll_two(fd0: Int32, fd1: Int32, timeoutMs: Int32, ready: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_sleep_ms")
internal func native_sleep_ms(ms: Int32)


@extern(.C, mangleName: "kestrel_monotonic_ms")
internal func native_monotonic_ms() -> Int64

@extern(.C, mangleName: "kestrel_executable_path")
internal func native_executable_path(buf: RawPointer, size: Int64) -> Int64

@extern(.C, mangleName: "kestrel_passwd_home")
internal func native_passwd_home() -> RawPointer

@extern(.C, mangleName: "kestrel_processor_count")
internal func native_processor_count() -> Int64

@extern(.C, mangleName: "kestrel_page_size")
internal func native_page_size() -> Int64

@extern(.C, mangleName: "kestrel_architecture")
internal func native_architecture() -> Int32

@extern(.C, mangleName: "kestrel_pointer_width")
internal func native_pointer_width() -> Int32

@extern(.C, mangleName: "kestrel_is_little_endian")
internal func native_is_little_endian() -> Int32

// ============================================================================
// Helpers
// ============================================================================

/// Copies a NUL-terminated C string into a `String`. `None` for a null
/// pointer. Bytes are copied as-is; no UTF-8 check.
internal func stringFromNative(raw: RawPointer) -> Optional[String] {
    if raw.isNull {
        return .None
    }
    .Some(String(from: CString(raw: raw.cast[UInt8]())))
}

/// Copies a NUL-terminated C string's bytes, without the terminator.
internal func bytesFromNative(raw: RawPointer) -> Array[UInt8] {
    let ptr = raw.cast[UInt8]();
    var bytes = Array[UInt8]();
    var i: Int64 = 0;
    while unsafe ptr.offset(by: i).read() != 0 {
        bytes.append(unsafe ptr.offset(by: i).read());
        i = i + 1
    }
    bytes
}

/// Reads the `index`th pointer of a C `char **` array.
internal func nativePointerAt(array: RawPointer, index: Int64) -> RawPointer {
    unsafe array.cast[RawPointer]().offset(by: index).read()
}

/// A small block of C `int32_t` cells for out-parameters. Freed on drop.
internal struct NativeInt32Cells: not Copyable {
    var pointer: Pointer[Int32]

    init(count: Int64) {
        let raw = unsafe std.ffi.malloc(count * 4);
        unsafe std.ffi.memset(raw, 0, count * 4);
        self.pointer = raw.cast[Int32]()
    }

    func asNative() -> RawPointer { self.pointer.asRaw() }

    func read(at index: Int64) -> Int32 { unsafe self.pointer.offset(by: index).read() }

    func write(value: Int32, at index: Int64) { unsafe self.pointer.offset(by: index).write(value) }

    deinit {
        unsafe std.ffi.free(self.pointer.asRaw());
    }
}

/// A NULL-terminated `char **` built from Kestrel strings, for argv and
/// envp. Owns the copies and frees them on drop.
internal struct NativeStringArray: not Copyable {
    var pointers: Array[RawPointer]

    init(from strings: Array[String]) {
        self.pointers = Array[RawPointer](capacity: strings.count + 1);
        for s in strings {
            self.pointers.append(s.toCString().raw.asRaw());
        }
        self.pointers.append(RawPointer.nullPointer());
    }

    /// The `char **` to hand to C. Valid while `self` is alive.
    func asNative() -> RawPointer {
        self.pointers.asPointer().asRaw()
    }

    deinit {
        for p in self.pointers {
            if not p.isNull {
                unsafe std.ffi.free(p);
            }
        }
    }
}
