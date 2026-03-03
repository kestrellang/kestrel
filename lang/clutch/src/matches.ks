// Parsed argument results

module clutch.matches

// ============================================================================
// ARG MATCHES
// ============================================================================

/// Stores the results of parsing command-line arguments.
///
/// Query results by argument name:
///
///     let output = matches.getValue(name: "output")
///     let verbose = matches.hasFlag(name: "verbose")
///     let count = matches.flagCount(name: "verbose")
///
public struct ArgMatches: Cloneable {
    /// Option names (parallel with values).
    var names: Array[String]
    /// Option values (parallel with names).
    var values: Array[String]

    /// Flags that were present.
    var flags: Array[String]

    /// Positional argument values in order.
    var positionals: Array[String]
    /// Positional argument names in order.
    var positionalNames: Array[String]

    /// The subcommand that was matched, if any.
    public var subcommand: Optional[String]

    /// ArgMatches for the matched subcommand (empty = none, 1 element = has submatches).
    public var submatches: Array[ArgMatches]

    public init() {
        self.names = Array[String]();
        self.values = Array[String]();
        self.flags = Array[String]();
        self.positionals = Array[String]();
        self.positionalNames = Array[String]();
        self.subcommand = .None;
        self.submatches = Array[ArgMatches]();
    }

    public func clone() -> ArgMatches {
        var m = ArgMatches();
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

    // --- internal mutating methods (used by parser) ---

    /// Stores a named option value.
    public mutating func setValue(name name: String, value value: String) {
        self.names.append(name);
        self.values.append(value)
    }

    /// Records that a flag was present.
    public mutating func setFlag(name name: String) {
        self.flags.append(name)
    }

    /// Stores a positional argument value.
    public mutating func setPositional(name name: String, value value: String) {
        self.positionalNames.append(name);
        self.positionals.append(value)
    }

    // --- public query methods ---

    /// Returns the value for a named option, or None.
    public func getValue(name name: String) -> Optional[String] {
        // Check named options
        var i: Int64 = 0;
        while i < self.names.count {
            if self.names(unchecked: i).equals(name) {
                return .Some(self.values(unchecked: i))
            }
            i = i + 1
        }
        // Check positionals
        var j: Int64 = 0;
        while j < self.positionalNames.count {
            if self.positionalNames(unchecked: j).equals(name) {
                return .Some(self.positionals(unchecked: j))
            }
            j = j + 1
        }
        .None
    }

    /// Returns true if a flag was present.
    public func hasFlag(name name: String) -> Bool {
        var i: Int64 = 0;
        while i < self.flags.count {
            if self.flags(unchecked: i).equals(name) {
                return true
            }
            i = i + 1
        }
        false
    }

    /// Returns the number of times a flag appeared (for -vvv counting).
    public func flagCount(name name: String) -> Int64 {
        var count: Int64 = 0;
        var i: Int64 = 0;
        while i < self.flags.count {
            if self.flags(unchecked: i).equals(name) {
                count = count + 1
            }
            i = i + 1
        }
        count
    }

    /// Returns all values for a multi-valued option.
    public func getAll(name name: String) -> Array[String] {
        var result = Array[String]();
        var i: Int64 = 0;
        while i < self.names.count {
            if self.names(unchecked: i).equals(name) {
                result.append(self.values(unchecked: i))
            }
            i = i + 1
        }
        result
    }
}
