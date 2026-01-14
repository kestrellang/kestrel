// libc bindings for memory operations
//
// These are thin wrappers around C standard library functions.

module std.ffi

import std.core.(Int)

// Memory allocation
@extern(.C)
public func malloc(size: Int) -> lang.ptr[lang.i8]

@extern(.C)
public func free(ptr: lang.ptr[lang.i8])

@extern(.C)
public func realloc(ptr: lang.ptr[lang.i8], size: Int) -> lang.ptr[lang.i8]

// Memory operations
@extern(.C)
public func memcpy(dest: lang.ptr[lang.i8], src: lang.ptr[lang.i8], n: Int) -> lang.ptr[lang.i8]

@extern(.C)
public func memmove(dest: lang.ptr[lang.i8], src: lang.ptr[lang.i8], n: Int) -> lang.ptr[lang.i8]

@extern(.C)
public func memset(dest: lang.ptr[lang.i8], c: Int, n: Int) -> lang.ptr[lang.i8]