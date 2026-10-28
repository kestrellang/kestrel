// SharedBox — the shared-ownership container contract the compiler codes
// against for implicit boxing (docs/design/shared-box.md).
//
// The compiler never emits retain/release/allocate sequences of its own: it
// instantiates the `@builtin(.SharedBox)` type at a payload type and touches it
// only through the requirements below, so a single lang-item swap retargets
// every client feature (escaping closures now; classes, `any` existentials and
// `indirect enum` payloads later) at once.

module std.memory

import std.core.(Bool, Cloneable, MutableIndirection)

/// The contract for a shared-ownership container the compiler may use for
/// implicit boxing: escaping closure environments, class storage, `any`
/// payloads, `indirect enum` payloads.
///
/// Conformers are *handles*: small, fixed-layout values pointing at managed
/// storage that owns one `Target`. Duplicating a handle (`clone()`) shares the
/// storage; dropping the last handle destroys the payload exactly once.
/// Most of the contract is inherited rather than invented — sharing is
/// `Cloneable.clone()`, releasing is the conformer's own `deinit`, and reading
/// the payload is `MutableIndirection.pointeeRef()` (which also supplies
/// transparent member access, so `box.field` reaches the payload's field).
/// Only creation, shared mutation, identity and uniqueness are new.
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
///     alias.pointeeRef()              // read the payload through the peel
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
///   over-release the payload.
/// - The handle's size and alignment do not depend on `Target`, so a handle
///   can be type-erased and can appear in a recursive layout.
/// - The payload is destroyed exactly once, when the last handle goes away.
/// - `sharedMutRef()` is the marked exception to value semantics; it is sound
///   while Kestrel is single-threaded.
public protocol SharedBox: Cloneable, MutableIndirection {
    /// Takes ownership of `value`, moves it into managed storage and returns
    /// the first handle onto it.
    ///
    /// `value` is a single-name parameter, so call sites are positional:
    /// `B(payload)`, never `B(value: payload)`.
    init(consuming value: Target)

    /// Mutable access to the payload through a *shared*, non-`mutating`
    /// handle — the interior-mutability primitive. Every handle onto the same
    /// storage observes the write; nothing forks and uniqueness is unchanged.
    ///
    /// This is the marked exception to value semantics, and the operation
    /// escaping closures and (later) classes are built on. Value-semantic
    /// clients — `any` payloads, `indirect enum`s — must not use it; they
    /// check `isUnique()` and fork instead.
    ///
    /// # Safety
    ///
    /// Sound only while Kestrel is single-threaded: the returned reference
    /// aliases storage that other handles can read at the same time. Writing
    /// through it while a copy-on-write client assumes value semantics breaks
    /// those semantics for every sharer.
    func sharedMutRef() -> &mutating Target

    /// Do the two handles refer to the same managed storage? Backs class
    /// identity (`===`).
    ///
    /// Required instead of exposing an address so that a moving collector can
    /// still conform. Handles derived from one `init` — by `clone()` or by the
    /// copy fold — are identical; handles from independent `init`s are not,
    /// however equal their payloads.
    func isIdentical(to other: Self) -> Bool

    /// `true` only when this handle is provably the sole owner. Backs
    /// copy-on-write forking.
    ///
    /// Implementations without cheap uniqueness information (a tracing
    /// collector) may conservatively return `false`; callers must therefore
    /// treat `false` as "fork before mutating" rather than as proof of
    /// sharing. A box that always answers `false` degrades to fork-always —
    /// slower, still correct.
    func isUnique() -> Bool
}
