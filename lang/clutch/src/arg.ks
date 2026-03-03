// Argument definition and builder

module clutch.arg

// ============================================================================
// ARG KIND
// ============================================================================

/// Distinguishes how an argument is parsed.
public enum ArgKind: Cloneable {
    /// A boolean flag (--verbose, -v). Presence means true.
    case Flag

    /// An option that takes a value (--output file).
    case Option

    /// A positional argument (unnamed, identified by position).
    case Positional

    public func clone() -> ArgKind {
        match self {
            .Flag => .Flag,
            .Option => .Option,
            .Positional => .Positional
        }
    }
}

// ============================================================================
// ARG
// ============================================================================

/// Defines a single argument for a command.
///
/// Use the mutating builder methods to configure the argument:
///
///     var arg = Arg(name: "output")
///     arg.short(flag: "o")
///     arg.placeholder(name: "FILE")
///     arg.help(text: "Output file path")
///     arg.isRequired()
///
public struct Arg: Cloneable {
    // --- identity ---
    public var name: String
    public var kind: ArgKind

    // --- flags ---
    public var longFlag: Optional[String]
    public var shortFlag: Optional[String]

    // --- metadata ---
    public var helpText: Optional[String]
    public var valueName: Optional[String]
    public var required: Bool
    public var defaultValue: Optional[String]

    /// Creates a new option argument with the given name.
    /// By default, the long flag is set to the name (e.g., name "output" => --output).
    public init(name name: String) {
        self.name = name;
        self.kind = ArgKind.Option;
        self.longFlag = .Some(name);
        self.shortFlag = .None;
        self.helpText = .None;
        self.valueName = .None;
        self.required = false;
        self.defaultValue = .None;
    }

    public func clone() -> Arg {
        var a = Arg(name: self.name.clone());
        a.kind = self.kind.clone();
        match self.longFlag {
            .Some(s) => a.longFlag = .Some(s.clone()),
            .None => a.longFlag = .None
        }
        match self.shortFlag {
            .Some(s) => a.shortFlag = .Some(s.clone()),
            .None => a.shortFlag = .None
        }
        match self.helpText {
            .Some(s) => a.helpText = .Some(s.clone()),
            .None => a.helpText = .None
        }
        match self.valueName {
            .Some(s) => a.valueName = .Some(s.clone()),
            .None => a.valueName = .None
        }
        a.required = self.required;
        match self.defaultValue {
            .Some(s) => a.defaultValue = .Some(s.clone()),
            .None => a.defaultValue = .None
        }
        a
    }

    // --- builder methods ---

    /// Sets the short flag character (e.g., "v" for -v).
    public mutating func short(flag flag: String) {
        self.shortFlag = .Some(flag);
    }

    /// Sets the long flag name (e.g., "verbose" for --verbose).
    public mutating func long(flag flag: String) {
        self.longFlag = .Some(flag);
    }

    /// Sets the help description for this argument.
    public mutating func help(text text: String) {
        self.helpText = .Some(text);
    }

    /// Sets the placeholder name shown in help (e.g., "FILE" for <FILE>).
    public mutating func placeholder(name name: String) {
        self.valueName = .Some(name);
    }

    /// Marks this argument as required.
    public mutating func isRequired() {
        self.required = true;
    }

    /// Sets a default value for this option.
    public mutating func defaultsTo(value value: String) {
        self.defaultValue = .Some(value);
    }

    /// Marks this argument as a boolean flag (no value).
    public mutating func asFlag() {
        self.kind = ArgKind.Flag;
    }

    /// Marks this argument as a positional argument.
    public mutating func asPositional() {
        self.kind = ArgKind.Positional;
        self.longFlag = .None;
        self.shortFlag = .None;
    }

    // --- queries ---

    /// Returns true if this argument is a flag.
    public func isFlag() -> Bool {
        match self.kind {
            .Flag => true,
            _ => false
        }
    }

    /// Returns true if this argument is a positional.
    public func isPositional() -> Bool {
        match self.kind {
            .Positional => true,
            _ => false
        }
    }

    /// Returns true if this argument is an option.
    public func isOption() -> Bool {
        match self.kind {
            .Option => true,
            _ => false
        }
    }
}
