// libc bindings for memory operations
//
// These are thin wrappers around C standard library functions.

module std.ffi

// Memory allocation
@extern(.C)
public func malloc(size: lang.i64) -> lang.ptr[lang.i8]

@extern(.C)
public func free(ptr: lang.ptr[lang.i8])

@extern(.C)
public func realloc(ptr: lang.ptr[lang.i8], size: lang.i64) -> lang.ptr[lang.i8]

// Memory operations
@extern(.C)
public func memcpy(dest: lang.ptr[lang.i8], src: lang.ptr[lang.i8], n: lang.i64) -> lang.ptr[lang.i8]

@extern(.C)
public func memmove(dest: lang.ptr[lang.i8], src: lang.ptr[lang.i8], n: lang.i64) -> lang.ptr[lang.i8]

@extern(.C)
public func memset(dest: lang.ptr[lang.i8], c: lang.i64, n: lang.i64) -> lang.ptr[lang.i8]
