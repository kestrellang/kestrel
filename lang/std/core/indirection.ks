// Indirection — transparent member access for smart pointers.
//
// A type conforming to `Indirection` lets `wrapper.member` reach through to
// its pointee: member access on the wrapper *peels* to the pointee, the same
// way `&T` already does. The peel is LAZY (it fires only after the wrapper's
// own members miss, so the wrapper always wins a name clash) and RECEIVER-ONLY
// (arguments never coerce — `f(wrapper)` where `f` wants the pointee stays a
// type error). Operators and other protocol conformances do NOT ride the peel;
// they forward via explicit `extend` (the `extend &T: P` pattern in `ref.ks`).

module std.core

import std.memory.(Pointer)

/// Opt-in transparent member access. `wrapper.foo` resolves through to
/// `wrapper.value.foo` when the wrapper has no `foo` of its own.
@builtin(.Indirection)
public protocol Indirection {
    /// The pointee type that member access forwards to. May be unsized: an
    /// existential box's `Target` is the raw existential it holds.
    @builtin(.IndirectionTarget)
    type Target: not Sized

    /// The pointee, lent in place. Member READS peel through its `ref`.
    var value: Target { ref }
}

/// A smart pointer whose pointee can also be mutated through the peel.
/// Writes (`wrapper.field = v`), compound assignments (`wrapper.n += 1`), and
/// `mutating`-method calls route through `value`'s `mutating ref`. A
/// read-only wrapper conforms to `Indirection` only; mutating through it is
/// rejected.
///
/// A `mutating ref` takes the wrapper mutably, so a copy-on-write wrapper can
/// fork its storage before handing out the mutable view.
@builtin(.MutableIndirection)
public protocol MutableIndirection: Indirection {
    /// The pointee, lent in place. Member WRITES / RMW / mutating methods
    /// peel through its `mutating ref`.
    var value: Target { ref mutating ref }
}

/// A box that can give up its payload: the single owner moves the payload
/// out and the box frees its storage. Consuming requirements called through
/// an existential, and `consuming x as? T`, go through this
/// (docs/plans/existentials/existentials-design.md §3). Shared boxes
/// (`RcBox`, `CowBox`) cannot conform — other handles may still see the
/// payload.
@builtin(.ConsumingIndirection)
public protocol ConsumingIndirection: MutableIndirection {
    /// Ends the box. `body` gets the payload's address and must move the
    /// payload out of it (or destroy it in place). The box then frees its
    /// storage without running the payload's deinit.
    ///
    /// A closure rather than a returned value because the payload's size may
    /// only be known at runtime; keeping "move out, then free" in one call
    /// means no caller can free the storage twice or forget to.
    @builtin(.ConsumingIndirectionConsumePointee)
    consuming func consumePointee[R](body: (Pointer[Target]) -> R) -> R where R: not Copyable, R: not Static
}
