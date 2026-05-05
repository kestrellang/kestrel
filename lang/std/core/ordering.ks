// Ordering enum for comparison results

module std.core

import std.text.(String, StringBuilder, FormatOptions, Formattable)

/// The three-valued result of a `Comparable.compare()` call.
///
/// `Ordering` is the lingua franca for comparison: types implementing
/// `Comparable` define a single `compare` returning this enum, and the
/// stdlib derives `<`, `<=`, `>`, `>=` on top. The `then` / `thenWith`
/// helpers make it easy to chain comparisons over multiple fields without
/// nested `if`s.
///
/// # Examples
///
/// ```
/// let cmp = a.compare(b);
/// match cmp {
///     .Less => "ascending",
///     .Equal => "tied",
///     .Greater => "descending"
/// }
///
/// // Chain field comparisons: by lastName, then firstName.
/// a.lastName.compare(b.lastName)
///     .then(a.firstName.compare(b.firstName))
/// ```
///
/// # Representation
///
/// A plain three-state enum with no payload — lowers to a small integer tag.
public enum Ordering: Equatable, Formattable {
    /// The receiver compared less than the argument.
    case Less
    /// The two values compared equal.
    case Equal
    /// The receiver compared greater than the argument.
    case Greater

    /// Equality on the orderings themselves: same variant ⇒ equal.
    public func isEqual(to other: Ordering) -> Bool {
        match (self, other) {
            (.Less, .Less) => true,
            (.Equal, .Equal) => true,
            (.Greater, .Greater) => true,
            _ => false
        }
    }

    /// Inverse of `isEqual`.
    public func isNotEqual(to other: Ordering) -> Bool {
        if self.isEqual(to: other) { false } else { true }
    }

    /// Swaps `Less` and `Greater`; leaves `Equal` alone. Useful for sorting
    /// in reverse without writing a second comparator.
    ///
    /// # Examples
    ///
    /// ```
    /// Ordering.Less.reverse()     // .Greater
    /// Ordering.Equal.reverse()    // .Equal
    /// ```
    public func reverse() -> Ordering {
        match self {
            .Less => .Greater,
            .Equal => .Equal,
            .Greater => .Less
        }
    }

    /// Tie-breaker chain: returns `self` if it is non-`Equal`, otherwise
    /// `other`. The eager form — both arguments are evaluated.
    ///
    /// # Examples
    ///
    /// ```
    /// Ordering.Equal.then(.Less)     // .Less
    /// Ordering.Greater.then(.Less)   // .Greater (self wins)
    /// ```
    public func then(other: Ordering) -> Ordering {
        match self {
            .Equal => other,
            _ => self
        }
    }

    /// Lazy variant of `then` — `compare` runs only when `self` is `Equal`.
    /// Prefer this when computing the secondary comparison is expensive.
    public func thenWith(compare: () -> Ordering) -> Ordering {
        match self {
            .Equal => compare(),
            _ => self
        }
    }

    /// Renders as `"Less"`, `"Equal"`, or `"Greater"`. With `debug` set,
    /// prefixes with the type name (`"Ordering.Less"`).
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        let value = match self {
            .Less => "Less",
            .Equal => "Equal",
            .Greater => "Greater"
        };
        if options.debug {
            writer.append("Ordering.");
            writer.append(value)
        } else {
            writer.append(value)
        }
    }
}
