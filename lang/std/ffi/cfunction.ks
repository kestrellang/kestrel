// CFunction: the address of a function C can call, for C APIs that take a
// callback (a sort comparator, a completion handler, a log hook).

module std.ffi

import std.core.(Bool, ExpressibleByNullLiteral)

/// The C-callable address of a function of type `F`.
///
/// Make one from an `@export(.C)` function (or an `@extern(.C)` one) with
/// `CFunction(f)`, and pass it to C wherever a C function pointer goes:
/// as an `@extern` argument or in an `FFISafe` struct field. It has exactly
/// the layout of a C function pointer, and C's `NULL` is the `null` literal.
///
/// Making one is safe. The C side decides when it is called; Kestrel can't
/// call through it yet.
///
/// # Examples
///
/// ```
/// @export(.C, mangleName: "compare_ints")
/// func compareInts(a: Pointer[()], b: Pointer[()]) -> Int32 { ... }
///
/// let cmp = CFunction(compareInts)   // CFunction[(Pointer[()], Pointer[()]) -> Int32]
/// unsafe qsort(base, count, 8, cmp)
/// ```
///
/// A closure carries no C address, so `f` must name the function directly
/// (E646). Pass a closure's state through the C API's `void* userdata`.
public struct CFunction[F]: FFISafe, ExpressibleByNullLiteral where F: not Copyable, F: not Static {
    fileprivate var _raw: lang.ptr[()]

    /// @name From Function
    /// The C-callable address of `f`, which must name an `@export(.C)` or
    /// `@extern(.C)` function: an export's C entry point, an import's C
    /// symbol.
    @builtin(.CFunctionInit)
    public init(f: F) {
        // Never runs: the compiler replaces every `CFunction(f)` with `f`'s
        // address.
        self._raw = lang.ptr_null[()]()
    }

    /// @name Null Literal
    /// C's `NULL` function pointer (`null`), for an optional callback left
    /// unset.
    public init() {
        self._raw = lang.ptr_null[()]()
    }

    /// Whether this is `null`.
    public var isNull: Bool { Bool(boolLiteral: lang.ptr_is_null(lang.cast_ptr[_, lang.i8](self._raw))) }

    /// The address as an untyped pointer.
    public var pointer: Pointer[()] { Pointer(raw: self._raw) }
}
