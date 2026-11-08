// Native bindings for std.files. Internal: the public API never exposes a
// raw pointer, and calls needing a target-specific struct layout go through
// `files_shims.c`.

module std.files

import std.numeric.(Int32, Int64, UInt8)
import std.memory.(Pointer, RawPointer)
import std.collections.(Array)
import std.text.(String)
import std.ffi.(CString, free)
import std.result.(Result)
import std.streams.(IoError)

@extern(.C, mangleName: "kestrel_file_open")
internal func native_open(path: RawPointer, options: Int32, mode: Int32) -> Int32

@extern(.C, mangleName: "kestrel_file_seek")
internal func native_seek(fd: Int32, offset: Int64, whence: Int32) -> Int64

@extern(.C, mangleName: "kestrel_file_sync")
internal func native_sync(fd: Int32) -> Int32

@extern(.C, mangleName: "kestrel_file_set_length")
internal func native_set_length(fd: Int32, length: Int64) -> Int32

@extern(.C, mangleName: "kestrel_path_metadata")
internal func native_path_metadata(path: RawPointer, follow: Int32, out: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_file_metadata")
internal func native_file_metadata(fd: Int32, out: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_directory_open")
internal func native_directory_open(path: RawPointer) -> RawPointer

@extern(.C, mangleName: "kestrel_directory_close")
internal func native_directory_close(dir: RawPointer)

@extern(.C, mangleName: "kestrel_directory_next")
internal func native_directory_next(dir: RawPointer, name: RawPointer, kind: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_make_temporary_directory")
internal func native_make_temporary_directory(template: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_path_canonical")
internal func native_canonical(path: RawPointer) -> RawPointer

@extern(.C, mangleName: "kestrel_path_read_link")
internal func native_read_link(path: RawPointer, buffer: RawPointer, capacity: Int64) -> Int64

@extern(.C, mangleName: "read")
internal func native_read(fd: Int32, buf: RawPointer, count: Int64) -> Int64

@extern(.C, mangleName: "write")
internal func native_write(fd: Int32, buf: RawPointer, count: Int64) -> Int64

@extern(.C, mangleName: "close")
internal func native_close(fd: Int32) -> Int32

@extern(.C, mangleName: "mkdir")
internal func native_mkdir(path: RawPointer, mode: Int32) -> Int32

@extern(.C, mangleName: "rmdir")
internal func native_rmdir(path: RawPointer) -> Int32

@extern(.C, mangleName: "unlink")
internal func native_unlink(path: RawPointer) -> Int32

@extern(.C, mangleName: "rename")
internal func native_rename(from: RawPointer, to: RawPointer) -> Int32

@extern(.C, mangleName: "symlink")
internal func native_symlink(target: RawPointer, link: RawPointer) -> Int32

@extern(.C, mangleName: "chmod")
internal func native_chmod(path: RawPointer, mode: Int32) -> Int32

/// Runs `body` with `path` as a NUL-terminated C string, freed afterwards.
/// `body` returns a C result: negative means failure, read from errno.
internal func withNativePath(path: Path, body: (RawPointer) -> Int32) -> Result[(), IoError] {
    let cpath = try path.toNativePath();
    let r = body(cpath.raw.asRaw());
    let error = IoError.last();
    cpath.free();
    if r < 0 { .Err(error) } else { .Ok(()) }
}
