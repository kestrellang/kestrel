// Core protocols
// Fundamental protocols for equality, comparison, hashing, and defaults.

module std.core

import std.core.(Less, LessOrEqual, Greater, GreaterOrEqual, NotEqual, Equal)
import std.text.(String)
import std.memory.(ArraySlice, Pointer)
import std.numeric.(UInt64, Int64)

/// Protocol for types whose values can be compared for equality.
///
/// `Equatable` is the semantic counterpart to the raw `Equal[Self]`
/// operator protocol: conformers implement `isEqual` returning `Bool`, and a
/// blanket extension below derives both `==` and `!=`. Most types should
/// reach for `Equatable` rather than `Equal` directly — the `Bool`
/// associated-type binding is wired up automatically.
///
/// # Examples
///
/// ```
/// public struct Point: Equatable {
///     public var x: Int64
///     public var y: Int64
///     public func isEqual(to other: Point) -> Bool {
///         self.x == other.x and self.y == other.y
///     }
/// }
///
/// Point(x: 1, y: 2) == Point(x: 1, y: 2)   // true
/// ```
public protocol Equatable {
    /// Returns `true` iff `self` and `other` are considered equal. Should
    /// be reflexive, symmetric, and transitive — `Hashable` requires equal
    /// values to hash equal, so don't drift from those laws.
    func isEqual(to other: Self) -> Bool
}

/// Protocol enabling `match` against custom types via the `case` pattern.
///
/// Conformers decide what "matches" means — for `Bool` and the integer
/// types it is straight equality; for ranges it is containment. The
/// compiler lowers `case <pattern> =>` to a `matches` call.
@builtin(.Matchable)
public protocol Matchable {
    /// Returns `true` if `other` matches the receiver.
    func matches(other: Self) -> Bool
}

/// Protocol enabling range patterns (`start..=end`, `..<end`, `start..`).
///
/// Split into three primitive comparisons rather than a single
/// "is in range" call so the compiler can lower partial ranges (e.g.
/// `..<10`) without synthesising a stand-in upper bound. The `Bound`
/// parameter lets a value be matched against bounds of a different type —
/// e.g. an `Int64` against `Char` bounds.
@builtin(.RangeMatchable)
public protocol RangeMatchable[Bound = Self] {
    /// Returns `true` when `self >= bound`. Powers `start..` patterns.
    @builtin(.RangeMatchableIsAtLeast)
    func isAtLeast(bound: Bound) -> Bool

    /// Returns `true` when `self <= bound`. Powers `..=end` patterns.
    @builtin(.RangeMatchableIsAtMost)
    func isAtMost(bound: Bound) -> Bool

    /// Returns `true` when `self < bound`. Powers `..<end` patterns.
    @builtin(.RangeMatchableIsBelow)
    func isBelow(bound: Bound) -> Bool
}

/// Protocol enabling array patterns (`[a, b]`, `[a, ..rest]`,
/// `[a, .., z]`, `[a, ..rest, z]`).
///
/// The compiler routes match-arm element access through `matchGet` and
/// rest-binding through `matchSlice` — they take `Int64` bounds the
/// compiler has already verified. A conformer may assume `0 <= index <
/// matchLength()` and `0 <= from <= to <= matchLength()` and skip its
/// own bounds checks; the conformance is unsafe to satisfy if those
/// invariants don't hold. `Array[T]` and `ArraySlice[T]` are the canonical
/// conformers.
@builtin(.ArrayMatchable)
public protocol ArrayMatchable {
    type Element

    /// Total number of elements available to match.
    @builtin(.ArrayMatchableMatchLength)
    func matchLength() -> Int64

    /// Returns the element at `index`. Caller (the compiler) guarantees
    /// `0 <= index < matchLength()` — implementations may skip bounds checks.
    @builtin(.ArrayMatchableMatchGet)
    func matchGet(index: Int64) -> Element

    /// Returns the slice `[from, to)`. Caller guarantees
    /// `0 <= from <= to <= matchLength()`.
    @builtin(.ArrayMatchableMatchSlice)
    func matchSlice(from: Int64, to: Int64) -> ArraySlice[Element]
}

/// Blanket extension giving every `Equatable` type the `==` and `!=`
/// operators with `Bool` results. Implements `notEqual` in terms of
/// `isEqual` so conformers only need to write the equality method.
extend Equatable: Equal[Self], NotEqual[Self] {
    type Equal.Output = Bool
    type NotEqual.Output = Bool

    /// Bridges `Equal.equal(to:)` to `Equatable.isEqual(to:)`.
    public func equal(to other: Self) -> Bool {
        self.isEqual(to: other)
    }

    /// Default `!=`: delegates to `==` so there's a single source of truth.
    public func notEqual(to other: Self) -> Bool {
        if self.equal(to: other) { false } else { true }
    }
}

/// Protocol for types with a total ordering.
///
/// Conformers implement a single `compare(other:) -> Ordering`; the
/// blanket extension below derives `<`, `<=`, `>`, `>=`, and `!=` (the
/// last shadowing the `Equatable` default since it can be cheaper via
/// `compare`). `Comparable` extends `Equatable`, so equal values and a
/// `compare` returning `.Equal` must agree.
///
/// # Examples
///
/// ```
/// public struct Version: Comparable {
///     public var major: Int64
///     public var minor: Int64
///     public func isEqual(to other: Version) -> Bool {
///         self.major == other.major and self.minor == other.minor
///     }
///     public func compare(other: Version) -> Ordering {
///         self.major.compare(other.major)
///             .then(self.minor.compare(other.minor))
///     }
/// }
/// ```
public protocol Comparable: Equatable {
    /// Returns the ordering of `self` relative to `other`. Must be a
    /// total order — for any `a`, `b`, `c` exactly one of `Less`,
    /// `Equal`, `Greater` holds, and the order is transitive.
    func compare(other: Self) -> Ordering
}

/// Blanket extension giving every `Comparable` type the four ordering
/// operators plus a sharper `!=`. All derived from a single `compare`
/// call to avoid repeated dispatch.
extend Comparable: Less[Self], LessOrEqual[Self], Greater[Self], GreaterOrEqual[Self] {
    type Less.Output = Bool
    type LessOrEqual.Output = Bool
    type Greater.Output = Bool
    type GreaterOrEqual.Output = Bool

    /// `<` derived from `compare`.
    public func lessThan(other: Self) -> Bool {
        self.compare(other) == Ordering.Less
    }

    /// `<=` derived from `compare`.
    public func lessThanOrEqual(other: Self) -> Bool {
        self.compare(other) != Ordering.Greater
    }

    /// `>` derived from `compare`.
    public func greaterThan(other: Self) -> Bool {
        self.compare(other) == Ordering.Greater
    }

    /// `>=` derived from `compare`.
    public func greaterThanOrEqual(other: Self) -> Bool {
        self.compare(other) != Ordering.Less
    }
}

/// Blanket extension exposing every `Comparable` type to range-pattern
/// matching. Each method goes through `compare` instead of `<` / `<=`
/// because direct comparison-operator dispatch can land in protocol
/// lookup loops during conformance checking — using `compare` keeps the
/// derivation grounded.
extend Comparable: RangeMatchable[Self] {
    /// `start..` lower-bound check, derived from `compare`.
    public func isAtLeast(bound: Self) -> Bool {
        self.compare(bound) != Ordering.Less
    }

    /// `..=end` upper-bound check, derived from `compare`.
    public func isAtMost(bound: Self) -> Bool {
        self.compare(bound) != Ordering.Greater
    }

    /// `..<end` upper-bound check, derived from `compare`.
    public func isBelow(bound: Self) -> Bool {
        self.compare(bound) == Ordering.Less
    }
}

/// Protocol for types whose values can be hashed.
///
/// `Hashable` extends `Equatable`: the contract is that `a == b` implies
/// `a.hash(into:)` and `b.hash(into:)` feed the same bytes to the hasher.
/// Violating this breaks `Set` and `Dictionary` — equal lookups won't
/// land on the equal stored value. The hasher is generic so the same
/// hash impl works across hashing algorithms (SipHash, FxHash, etc.).
///
/// # Examples
///
/// ```
/// public struct Tag: Hashable {
///     public var name: String
///     public func isEqual(to other: Tag) -> Bool { self.name == other.name }
///     public func hash[H](mutating into hasher: H) where H: Hasher {
///         self.name.hash(into: hasher)
///     }
/// }
/// ```
public protocol Hashable: Equatable {
    /// Feeds this value's bytes into `hasher`. Must be deterministic
    /// across calls and consistent with `isEqual`.
    func hash[H](mutating into hasher: H) where H: Hasher
}

/// Protocol for hash algorithm implementations consumed by `Hashable`.
///
/// The contract is the same as Rust / Swift: `Hashable`-conforming types
/// `write` their bytes into the hasher; the hasher accumulates state
/// and emits a `UInt64` digest on `finish()`. Used by `Set`,
/// `Dictionary`, and any structure that wants stable hashes.
public protocol Hasher {
    /// Mixes `bytes` into the running hash state.
    mutating func write(bytes: ArraySlice[UInt8])
    /// Returns the finalised hash. After calling `finish` the hasher's
    /// state is unspecified — don't reuse it.
    mutating func finish() -> UInt64
}

/// Protocol for types with a meaningful zero/default value.
///
/// `Defaultable` is what `T()` resolves to when no other init is
/// chosen. Conform when there's an obvious default: `0` for numbers,
/// `""` for strings, the empty collection for containers. Don't
/// conform just to satisfy a generic bound — the absence of a default
/// is information.
public protocol Defaultable {
    /// @name Default
    /// Builds the default-valued instance.
    init()
}

// Note: Formattable protocol is now in std.text/format.ks
