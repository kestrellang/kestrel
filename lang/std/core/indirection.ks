// Indirection — transparent member access for smart pointers.
//
// A type conforming to `Indirection` lets `wrapper.member` reach through to
// the value it holds: member access on the wrapper *peels* to that value, the same
// way `&T` already does. The peel is LAZY (it fires only after the wrapper's
// own members miss, so the wrapper always wins a name clash). A BORROWED call
// argument peels too: `f(wrapper)` where `f` takes the value (`x: T`,
// `x: &T`, `mutating x: T`) passes the value in place. A `consuming`
// argument, a binding and a return never coerce. Operators and other protocol
// conformances do NOT ride the peel; they forward via explicit `extend`
// (the `extend &T: P` pattern in `ref.ks`).

module std.core

import std.memory.(Pointer)

/// Opt-in transparent member access. `wrapper.foo` resolves through to
/// `wrapper.value.foo` when the wrapper has no `foo` of its own.
@builtin(.Indirection)
public protocol Indirection {
    /// The type of the value member access forwards to. May be unsized: an
    /// existential box's `Target` is the raw existential it holds.
    @builtin(.IndirectionTarget)
    type Target: not Sized

    /// The value, lent in place. Member READS peel through its `ref`.
    var value: Target { ref }
}

/// A smart pointer whose value can also be mutated through the peel.
/// Writes (`wrapper.field = v`), compound assignments (`wrapper.n += 1`), and
/// `mutating`-method calls route through `value`'s `mutating ref`. A
/// read-only wrapper conforms to `Indirection` only; mutating through it is
/// rejected.
///
/// A `mutating ref` takes the wrapper mutably, so a copy-on-write wrapper can
/// fork its storage before handing out the mutable view.
@builtin(.MutableIndirection)
public protocol MutableIndirection: Indirection {
    /// The value, lent in place. Member WRITES / RMW / mutating methods
    /// peel through its `mutating ref`.
    var value: Target { ref mutating ref }
}

/// A box that can give up its value: the single owner moves the value out
/// and the box frees its storage. Consuming requirements called through an
/// existential, and `consuming x as? T`, go through this
/// (docs/plans/existentials/existentials-design.md §3). Shared boxes
/// (`RcBox`, `CowBox`) cannot conform — other handles may still see the
/// value.
@builtin(.ConsumingIndirection)
public protocol ConsumingIndirection: MutableIndirection {
    /// Ends the box. `body` gets the value's address and must move the
    /// value out of it (or destroy it in place). The box then frees its
    /// storage without running the value's deinit.
    ///
    /// A closure rather than a returned value because the value's size may
    /// only be known at runtime; keeping "move out, then free" in one call
    /// means no caller can free the storage twice or forget to.
    @builtin(.ConsumingIndirectionConsumeValue)
    consuming func consumeValue[R](body: (Pointer[Target]) -> R) -> R where R: not Copyable, R: not Static
}

extend ConsumingIndirection where Target: Sized {
    /// Ends the box and hands its value back: `consumeValue` with a body
    /// that moves the value out. Boxed `indirect enum` cases are unboxed
    /// through this when a match takes them apart.
    @builtin(.ConsumingIndirectionTakeValue)
    public consuming func takeValue() -> Target {
        self.consumeValue { (p) in unsafe p.take() }
    }
}
