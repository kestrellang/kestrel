// FFI (Foreign Function Interface) support
//
// Types that conform to FFISafe can be passed across FFI boundaries
// to/from C functions declared with @extern(.C).
//
// Primitive types (Int8, Int16, Int32, Int64, UInt8, UInt16, UInt32, UInt64,
// Float32, Float64, Bool) and Pointer[T] where T: FFISafe conform to FFISafe.
//
// Structs can opt into FFISafe by declaring conformance:
//   struct MyStruct: FFISafe { ... }
// All fields of an FFISafe struct must also be FFISafe.
//
// Tuples of FFISafe types are also FFISafe.

module std.ffi

/// Marker protocol for types that can safely cross FFI boundaries.
/// Types must have a well-defined memory layout compatible with C.
/// Primitive types and pointers to FFISafe types conform automatically.
/// Custom structs can conform if all their fields are FFISafe.
@builtin(.FFISafe)
public protocol FFISafe {}
