module regex

/// Why a pattern failed to compile.
///
/// # Examples
///
/// ```
/// match Regex.compile("a(b") {
///     .Ok(_) => {},
///     .Err(e) => print(e.description)   // "unclosed group at byte 1"
/// }
/// ```
public struct RegexError {
    /// What went wrong: "unclosed group", "bad escape", ...
    public let message: String;
    /// The byte offset in the pattern where the problem was found.
    public let position: Int64;

    public init(message message: String, position position: Int64) {
        self.message = message;
        self.position = position;
    }

    public var description: String { "\(self.message) at byte \(self.position)" }
}
