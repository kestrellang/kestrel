module clutch.matches

/// The result of a successful parse — holds every matched value.
///
/// `ArgumentMatches` is produced by `Command.parse(from:)` on success.
/// Query it by argument name to retrieve option values, check flag
/// presence, and access subcommand results.
///
/// Option and positional values are stored as strings; the caller is
/// responsible for any type conversion (e.g., `Int64.parse`).
///
/// # Representation
///
/// Internally, named options are stored as two parallel arrays (`names`
/// and `values`), flags as a flat string array (duplicates allowed, for
/// `-vvv` counting), and positionals as two parallel arrays
/// (`positionalNames` and `positionals`). Subcommand results live in
/// `subcommand` (the matched name) and `submatches` (the child's
/// `ArgumentMatches`).
///
/// # Examples
///
/// ```
/// let cmd = Command("mycli")
///     .argument(Argument("output").short("o").help("Output path"))
///     .argument(Argument("verbose").short("v").toFlag());
///
/// match cmd.parse(from: ["-v", "--output", "out.txt"]) {
///     .Ok(matches) => {
///         matches.value(for: "output");   // .Some("out.txt")
///         matches.hasFlag("verbose");     // true
///     },
///     .Err(e) => eprintln(e.description())
/// }
/// ```
public struct ArgumentMatches: Cloneable {
    /// Named option keys, parallel with `values`.
    var names: Array[String]

    /// Named option values, parallel with `names`.
    var values: Array[String]

    /// Flag names that were present. May contain duplicates when a flag
    /// appears more than once (e.g., `-vvv` adds `"verbose"` three
    /// times).
    var flags: Array[String]

    /// Positional argument values in the order they were parsed.
    var positionals: Array[String]

    /// Positional argument names in the order they were parsed,
    /// parallel with `positionals`.
    var positionalNames: Array[String]

    /// The subcommand that was matched, or `.None` if no subcommand
    /// appeared in the input.
    public var subcommand: Optional[String]

    /// `ArgumentMatches` for the matched subcommand. Empty when no
    /// subcommand was matched; contains exactly one element otherwise.
    public var submatches: Array[ArgumentMatches]

    /// @name Default
    /// Creates an empty result set with no matched values.
    public init() {
        self.names = Array[String]();
        self.values = Array[String]();
        self.flags = Array[String]();
        self.positionals = Array[String]();
        self.positionalNames = Array[String]();
        self.subcommand = .None;
        self.submatches = Array[ArgumentMatches]();
    }

    /// Creates a deep copy of all matched data.
    public func clone() -> ArgumentMatches {
        var m = ArgumentMatches();
        m.names = self.names.clone();
        m.values = self.values.clone();
        m.flags = self.flags.clone();
        m.positionals = self.positionals.clone();
        m.positionalNames = self.positionalNames.clone();
        match self.subcommand {
            .Some(s) => m.subcommand = .Some(s.clone()),
            .None => {}
        }
        m.submatches = self.submatches.clone();
        m
    }

    // --- internal mutating methods (used by the parser) ---

    /// Records a named option value. Called by the parser when it
    /// resolves `--key value` or `--key=value`.
    public mutating func setValue(name name: String, value value: String) {
        self.names.append(name);
        self.values.append(value)
    }

    /// Records that a flag was present. Called once per flag occurrence
    /// so that `-vvv` registers three entries.
    public mutating func setFlag(name name: String) {
        self.flags.append(name)
    }

    /// Records a positional argument value. Called by the parser in
    /// the order positional tokens appear.
    public mutating func setPositional(name name: String, value value: String) {
        self.positionalNames.append(name);
        self.positionals.append(value)
    }

    // --- public query methods ---

    /// Returns the value for the given argument name, or `.None`.
    ///
    /// Searches named options first, then positionals. If the same name
    /// was registered as both an option and a positional (unusual), the
    /// option takes precedence.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given: mycli --output out.txt hello.txt
    /// matches.value(for: "output");  // .Some("out.txt")
    /// matches.value(for: "file");    // .Some("hello.txt")  (positional)
    /// matches.value(for: "missing"); // .None
    /// ```
    public func value(for name: String) -> Optional[String] {
        for i in 0..<self.names.count {
            if self.names(unchecked: i) == name {
                return .Some(self.values(unchecked: i))
            }
        }
        for i in 0..<self.positionalNames.count {
            if self.positionalNames(unchecked: i) == name {
                return .Some(self.positionals(unchecked: i))
            }
        }
        .None
    }

    /// Returns `true` if the named flag appeared at least once.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given: mycli -v
    /// matches.hasFlag("verbose");  // true
    /// matches.hasFlag("quiet");    // false
    /// ```
    public func hasFlag(name: String) -> Bool {
        for flag in self.flags {
            if flag == name { return true }
        }
        false
    }

    /// Returns the number of times the named flag appeared.
    ///
    /// Useful for flags like `-vvv` where repetition encodes a level.
    /// Returns `0` if the flag was never seen.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given: mycli -vvv
    /// matches.flagCount(for: "verbose");  // 3
    /// matches.flagCount(for: "quiet");    // 0
    /// ```
    public func flagCount(for name: String) -> Int64 {
        var count: Int64 = 0;
        for flag in self.flags {
            if flag == name { count = count + 1; }
        }
        count
    }

    /// Returns all values for a multi-valued option.
    ///
    /// When the same `--key value` option appears more than once, each
    /// occurrence is recorded separately. This method collects them all
    /// in order. Returns an empty array if the name was never matched.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given: mycli --include foo --include bar
    /// matches.allValues(for: "include");  // ["foo", "bar"]
    /// matches.allValues(for: "missing");  // []
    /// ```
    public func allValues(for name: String) -> Array[String] {
        var result = Array[String]();
        for i in 0..<self.names.count {
            if self.names(unchecked: i) == name {
                result.append(self.values(unchecked: i));
            }
        }
        result
    }
}
