// Reference-containment marker protocol

module std.core

/// Marker protocol for types that transitively contain no reference (`&T`).
///
/// `Static` values have no borrowed parts, so they may live anywhere: heap
/// storage, globals, and escaping closures. Conformance is structural — the
/// compiler computes it from a type's stored fields and enum payloads; it is
/// never declared. Every generic type parameter carries an implicit
/// `T: Static` bound; relax it with `where T: not Static` to accept
/// reference-bearing types (the param then accepts both Static and
/// non-Static arguments, like `not Copyable`). A nominal type declares
/// `: not Static` to mark itself reference-bearing.
@builtin(.Static)
public protocol Static {}
