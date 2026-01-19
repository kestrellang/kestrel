// libc bindings for memory operations
//
// These are thin wrappers around C standard library functions.

module std.ffi

// Memory allocation
@extern(.C, mangleName: "malloc_debug")
public func malloc(consuming size: lang.i64) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "free_debug")
public func free(consuming ptr: lang.ptr[lang.i8])

@extern(.C, mangleName: "realloc_debug")
public func realloc(consuming ptr: lang.ptr[lang.i8], consuming size: lang.i64) -> lang.ptr[lang.i8]

// Memory operations
@extern(.C, mangleName: "memcpy_debug")
public func memcpy(
    consuming dest: lang.ptr[lang.i8],
    consuming src: lang.ptr[lang.i8],
    consuming n: lang.i64
) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "memmove_debug")
public func memmove(
    consuming dest: lang.ptr[lang.i8],
    consuming src: lang.ptr[lang.i8],
    consuming n: lang.i64
) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "memset_debug")
public func memset(
    consuming dest: lang.ptr[lang.i8],
    consuming c: lang.i64,
    consuming n: lang.i64
) -> lang.ptr[lang.i8]
