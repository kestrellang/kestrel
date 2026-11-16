// Box and SharedBox — the container contracts the compiler codes against
// for implicit boxing (docs/design/shared-box.md).
//
// The compiler never emits retain/release/allocate sequences of its own. Each
// boxing feature names a tagged alias in `box_bindings.ks`; the compiler
// instantiates that alias's box at a value type and touches it only through
// the requirements below, so std can retarget a feature (or a whole role) by
// editing one alias.

module std.memory

import std.core.(Bool, Cloneable, Indirection, MutableIndirection)

/// A box: a handle onto one `Target` stored behind a pointer, built from a
/// value. Every box conforms: `RcBox`, `CowBox`, `OwnedBox`.
///
/// `any[B] P` is `B[<raw existential>]`
/// (docs/plans/existentials/existentials-design.md §3), so beyond conforming,
/// `B` must have exactly one type parameter without a default (others, like
/// an allocator, must have one), declared `not Sized`, and store it only
/// behind a pointer, so the compiler can widen `B[T]` to
/// `B[<raw existential>]` (E402 checks this).
@builtin(.Box)
public protocol Box: Indirection {
    /// Takes ownership of `value`, moves it into the box's storage and
    /// returns the first handle onto it. Only ever called at a sized
    /// `Target`; boxing an existential converts the result afterwards.
    ///
    /// `value` is a single-name parameter, so call sites are positional:
    /// `B(value)`, never `B(value: value)`.
    @builtin(.BoxInit)
    init(consuming value: Target)

    /// `true` only when this handle is provably the sole owner of the boxed
    /// value. Backs copy-on-write forking.
    ///
    /// A single-owner box always answers `true`. Implementations without
    /// cheap uniqueness information (a tracing collector) may conservatively
    /// return `false`; callers must therefore treat `false` as "fork before
    /// mutating" rather than as proof of sharing. A box that always answers
    /// `false` degrades to fork-always — slower, still correct.
    var isUnique: Bool { get }
}

/// A box whose handles alias: the shared-ownership contract behind escaping
/// closure environments, class storage and plain `any` values.
///
/// Conformers are *handles*: small, fixed-layout values pointing at managed
/// storage that owns one `Target`. Duplicating a handle (`clone()`) shares the
/// storage; dropping the last handle destroys the boxed value exactly once.
/// Most of the contract is inherited rather than invented — sharing is
/// `Cloneable.clone()`, releasing is the conformer's own `deinit`, and reading
/// the boxed value is `MutableIndirection.value` (which also supplies
/// transparent member access, so `box.field` reaches the value's field), and
/// creation and uniqueness come from `Box`. Only shared mutation and identity
/// are new.
///
/// The mechanism-neutral name belongs to this protocol; each conformer says
/// what it actually does. `RcBox` is the stdlib default (non-atomic
/// refcounting, deterministic last-release cleanup, no strong-cycle
/// collection); a future `ArcBox` or `GcBox` would make different promises.
/// This protocol deliberately promises neither determinism nor cycle
/// collection — those are properties of the binding, documented there.
///
/// # Examples
///
/// ```
/// // Generic over any conforming box; uses only SharedBox requirements.
/// func roundTrip[B](value: Int64) -> Int64 where B: SharedBox, B.Target = Int64 {
///     let box: B = B(value);          // init(consuming value: Target)
///     let alias = box.clone();        // shares the managed storage
///     box.isIdentical(to: alias);     // true — one storage, two handles
///     alias.value                     // read the boxed value in place
/// }
/// ```
///
/// # Memory Model
///
/// A conformer is a handle onto storage that is owned collectively by every
/// live handle. Lifecycle is ordinary value semantics — no compiler hooks are
/// involved: the aggregate copy fold routes a struct/enum copy through
/// `clone()`, so a struct holding a handle becomes `Cloneable` and shares on
/// copy for free, and the drop machinery releases handles like any other
/// value.
///
/// # Guarantees
///
/// - Handles are never bitwise-`Copyable` (the `Cloneable` refinement says
///   so): a bit-copied handle would skip the share operation and
///   over-release the boxed value.
/// - The handle's size and alignment do not depend on `Target`, so a handle
///   can be type-erased and can appear in a recursive layout.
/// - The boxed value is destroyed exactly once, when the last handle goes away.
/// - `sharedMutRef()` is the marked exception to value semantics; it is sound
///   while Kestrel is single-threaded.
@builtin(.SharedBox)
public protocol SharedBox: Box, Cloneable, MutableIndirection {
    /// Mutable access to the boxed value through a *shared*, non-`mutating`
    /// handle — the interior-mutability primitive. Every handle onto the same
    /// storage observes the write; nothing forks and uniqueness is unchanged.
    ///
    /// This is the marked exception to value semantics, and the operation
    /// escaping closures and (later) classes are built on. Value-semantic
    /// clients (copy-on-write wrappers) must not use it; they check
    /// `isUnique` and fork instead.
    ///
    /// # Safety
    ///
    /// Sound only while Kestrel is single-threaded: the returned reference
    /// aliases storage that other handles can read at the same time. Writing
    /// through it while a copy-on-write client assumes value semantics breaks
    /// those semantics for every sharer.
    @builtin(.SharedBoxSharedMutRef)
    func sharedMutRef() -> &mutating Target

    /// Do the two handles refer to the same managed storage? Backs class
    /// identity (`===`).
    ///
    /// Required instead of exposing an address so that a moving collector can
    /// still conform. Handles derived from one `init` — by `clone()` or by the
    /// copy fold — are identical; handles from independent `init`s are not,
    /// however equal their values.
    func isIdentical(to other: Self) -> Bool
}
