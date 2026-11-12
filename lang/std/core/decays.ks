// Target-directed conversion from a borrowed view to its owned type.

module std.core

/// A type an unsized borrowed view converts into where the type is named.
///
/// A reference to an unsized view (`&StringSlice`, `&Slice[T]`) never decays
/// on its own: an unannotated `let` keeps the view. Where the target names an
/// owned type that conforms to `DecaysFrom[View]` (an annotation, a
/// parameter, a field, a return type, an assignment target, or an annotated
/// collection element) the compiler builds that type from the view by
/// calling `decaying(from:)`. A `mutating` parameter never converts: writes
/// would land in a temporary.
///
/// # Examples
///
/// ```
/// let text = "  hi  ";
/// let word = text.trimmed();          // &StringSlice, borrows `text`
/// let owned: String = text.trimmed(); // String, via DecaysFrom
/// ```
@builtin(.DecaysFromProtocol)
public protocol DecaysFrom[Source] where Source: not Sized, Source: not Static, Source: not Copyable {
    /// Builds an owned value holding a copy of `value`'s contents. Called by
    /// the compiler at the conversion site.
    @sugarOnly
    @builtin(.DecaysFromMethod)
    static func decaying(from value: &Source) -> Self
}
