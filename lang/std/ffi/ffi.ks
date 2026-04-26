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

/// Marker protocol for types that can cross an `@extern(.C)` boundary.
///
/// `FFISafe` is empty — conformance is purely a contract that the
/// type's in-memory layout matches what a C function expects.
/// Conformance is *not* automatically transitive at the type level:
/// the compiler checks `FFISafe` constraints separately on each
/// generic instantiation. The conformance rules:
///
///   - Primitive numeric types and `Bool` conform automatically.
///   - `Pointer[T]` conforms iff `T: FFISafe`.
///   - Tuples of `FFISafe` types conform.
///   - User structs may opt in (`struct Foo: FFISafe`); every field
///     must itself be `FFISafe`.
///
/// String and other heap-managed types do **not** conform — convert
/// at the boundary with `String.toCString()` and pass `CString`.
///
/// # Examples
///
/// ```
/// struct Point: FFISafe { var x: Int32; var y: Int32 }
///
/// @extern(.C, mangleName: "process_point")
/// func processPoint(p: Point) -> Int32
/// ```
@builtin(.FFISafe)
public protocol FFISafe {}
