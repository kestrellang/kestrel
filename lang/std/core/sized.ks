// Compile-time-size marker protocol (existentials).

module std.core

/// Marker protocol for types whose size is known at compile time.
///
/// Every type is `Sized` except the raw existential an `any P` box holds,
/// and a struct whose last stored field is an unsized type parameter
/// instantiated at an unsized type. Conformance is structural — computed,
/// never declared. Every generic type parameter carries an implicit
/// `T: Sized` bound; relax it with `where T: not Sized` (a box type, or a
/// storage struct with an unsized tail). Inside such a declaration `T` can
/// only be used behind a pointer or as the last stored field; members that
/// take or return `T` by value go in an `extend … where T: Sized` extension
/// (docs/plans/existentials/existentials-design.md §2).
@builtin(.Sized)
public protocol Sized {}
