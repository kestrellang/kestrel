// Optional chaining protocol
// `a?.b` asks `a` for its payload; with none, the rest of the chain is skipped.

module std.core

import std.result.(Optional)

/// Raw protocol backing the `?.` operator.
///
/// `a?.rest` evaluates `a` once and asks it for its payload. With none, the
/// rest of the postfix chain is skipped and the whole expression is `.None`;
/// otherwise the chain continues on the payload and its result is wrapped in
/// `Optional` (or flattened, when it already is one). The chain's result is
/// always `Optional`, whatever the conforming type is.
///
/// A conforming type hands out its payload three ways; the compiler picks
/// one per chain from what the chain does with it.
///
/// # Examples
///
/// ```
/// let theme = user?.profile?.settings.theme ?? Theme.default;
/// inventory?.items.push(item);       // mutates in place
/// let n = parse(text)?.count;        // `Result` chains on its `Ok` value
/// ```
@builtin(.OptionalChainProtocol)
public protocol OptionalChain {
    /// The type the chain continues on.
    type Wrapped

    /// Read-only chains: `a?.count`, `a?.describe()`.
    @builtin(.OptionalChainBorrowMethod)
    func chainBorrow() -> Optional[&Wrapped]

    /// Chains that mutate the payload: `a?.push(x)`, `a?.n = 1`.
    @builtin(.OptionalChainMutateMethod)
    mutating func chainMutate() -> Optional[&mutating Wrapped]

    /// Chains on a temporary: `load()?.consume()`.
    @builtin(.OptionalChainTakeMethod)
    consuming func chainTake() -> Optional[Wrapped]
}
