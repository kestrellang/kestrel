// libc bindings for memory operations
//
// These are thin wrappers around C standard library functions.

module std.ffi

import std.memory.(RawPointer)
import std.numeric.(Int64, Int32)

// Memory allocation

/// Wraps `malloc(3)` — allocates `size` bytes of uninitialised memory.
///
/// Returns a pointer to the start of the block, or null on failure.
/// The memory is **uninitialised** — read it only after writing
/// every byte you intend to use, or follow up with `memset` /
/// `calloc` (not exposed here). Free with `free`.
///
/// # Safety
///
/// The returned pointer is raw. Callers are responsible for not
/// reading uninitialised bytes, not exceeding `size`, and pairing
/// every successful call with exactly one `free`.
///
/// # Examples
///
/// ```
/// let buf = malloc(1024);
/// // ... use buf ...
/// free(buf);
/// ```
@extern(.C, mangleName: "malloc")
public func malloc(consuming size: Int64) -> RawPointer

/// Wraps `free(3)` — releases memory previously returned by `malloc` / `realloc`.
///
/// Calling `free` on a null pointer is defined as a no-op. Calling
/// it on any other pointer that was not produced by these
/// allocators (or has already been freed) is undefined behaviour.
///
/// # Safety
///
/// `ptr` must be either null or the original pointer returned by a
/// previous `malloc` / `realloc`. After `free`, the pointer is
/// dangling — do not read, write, or free it again.
@extern(.C, mangleName: "free")
public func free(consuming ptr: RawPointer)

/// Wraps `realloc(3)` — resizes a previously-`malloc`'d block.
///
/// May return the same pointer or a new one; either way, the original
/// pointer becomes invalid. Returns null on failure, in which case
/// the original block is **not** freed — capture the return value
/// before reassigning. Bytes beyond the old size in a grown block
/// are uninitialised.
///
/// # Safety
///
/// `ptr` must be null or a pointer from a previous `malloc` /
/// `realloc`. After a successful call, only the returned pointer is
/// valid; after a failed call, only the original pointer is valid.
///
/// # Examples
///
/// ```
/// var buf = malloc(64);
/// let bigger = realloc(buf, 256);
/// // buf is no longer valid; use bigger
/// ```
@extern(.C, mangleName: "realloc")
public func realloc(consuming ptr: RawPointer, consuming size: Int64) -> RawPointer

// Memory operations

/// Wraps `memcpy(3)` — copies `n` bytes from `src` to `dest`.
///
/// **Source and destination must not overlap** — use `memmove` if
/// they might. Returns `dest` for convenience. No bounds checking;
/// the caller must ensure both regions are valid for `n` bytes.
///
/// # Safety
///
/// `src` and `dest` must be valid for `n` bytes each, and the two
/// regions must not overlap.
@extern(.C, mangleName: "memcpy")
public func memcpy(
    consuming dest: RawPointer,
    consuming src: RawPointer,
    consuming n: Int64
) -> RawPointer

/// Wraps `memmove(3)` — copies `n` bytes from `src` to `dest`, allowing overlap.
///
/// Slightly slower than `memcpy` because it has to detect direction;
/// use `memcpy` when you know the regions are disjoint. Returns
/// `dest`.
///
/// # Safety
///
/// `src` and `dest` must each be valid for `n` bytes. Overlap is
/// permitted.
@extern(.C, mangleName: "memmove")
public func memmove(
    consuming dest: RawPointer,
    consuming src: RawPointer,
    consuming n: Int64
) -> RawPointer

/// Wraps `memset(3)` — fills `n` bytes starting at `dest` with the low byte of `c`.
///
/// `c` is widened to `i64` to match the libc signature, but only
/// the low 8 bits are used; pass `0` to zero the region. Returns
/// `dest`.
///
/// # Safety
///
/// `dest` must be valid for `n` bytes of writes.
@extern(.C, mangleName: "memset")
public func memset(
    consuming dest: RawPointer,
    consuming c: Int64,
    consuming n: Int64
) -> RawPointer

/// Wraps `memmem(3)` — locates the first occurrence of the `needleLen`-byte
/// `needle` in the `haystackLen`-byte `haystack`.
///
/// Returns a pointer to the start of the match, or null if not found.
/// `needleLen == 0` returns `haystack` (per glibc/macOS conventions —
/// callers should check this before calling). Available on Linux and
/// macOS; not on Windows.
///
/// # Safety
///
/// `haystack` must be valid for `haystackLen` bytes; `needle` must be
/// valid for `needleLen` bytes.
@extern(.C, mangleName: "memmem")
public func memmem(
    consuming haystack: RawPointer,
    consuming haystackLen: Int64,
    consuming needle: RawPointer,
    consuming needleLen: Int64
) -> RawPointer

/// Wraps `memcmp(3)` — compares the first `n` bytes of `a` and `b`.
///
/// Returns a negative value if the first differing byte in `a` is less
/// than the corresponding byte in `b`, zero if all bytes are equal,
/// positive otherwise. Comparison is unsigned, byte-by-byte.
///
/// # Safety
///
/// Both `a` and `b` must be valid for `n` bytes.
@extern(.C, mangleName: "memcmp")
public func memcmp(
    consuming a: RawPointer,
    consuming b: RawPointer,
    consuming n: Int64
) -> Int32
