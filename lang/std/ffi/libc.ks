// libc bindings for memory operations
//
// These are thin wrappers around C standard library functions.

module std.ffi

// Memory allocation
@extern(.C, mangleName: "malloc")
public func malloc(consuming size: lang.i64) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "free")
public func free(consuming ptr: lang.ptr[lang.i8])

@extern(.C, mangleName: "realloc")
public func realloc(consuming ptr: lang.ptr[lang.i8], consuming size: lang.i64) -> lang.ptr[lang.i8]

// Memory operations
@extern(.C, mangleName: "memcpy")
public func memcpy(
    consuming dest: lang.ptr[lang.i8],
    consuming src: lang.ptr[lang.i8],
    consuming n: lang.i64
) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "memmove")
public func memmove(
    consuming dest: lang.ptr[lang.i8],
    consuming src: lang.ptr[lang.i8],
    consuming n: lang.i64
) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "memset")
public func memset(
    consuming dest: lang.ptr[lang.i8],
    consuming c: lang.i64,
    consuming n: lang.i64
) -> lang.ptr[lang.i8]
