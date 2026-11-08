/// Regular expressions in pure Kestrel.
///
/// Patterns use the familiar Perl/RE2 syntax without backreferences or
/// lookaround, and every search runs in time linear in the input (a Pike
/// VM, like RE2 and Rust's `regex`), so no pattern can make a search
/// blow up. Positions are byte offsets into the UTF-8 text.
///
/// # Examples
///
/// ```
/// import regex.(Regex)
///
/// let date = try Regex.compile("(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})");
/// if let .Some(c) = date.captures(in: "due 2026-11-05") {
///     print(c.named("year")?.text ?? "");
/// }
/// let tidy = Regex.compile("\\s+")?.replacingAll(in: text, with: " ");
/// ```
module regex
