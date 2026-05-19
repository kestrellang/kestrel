module clutch.argument

/// Distinguishes how an argument is parsed.
///
/// Every `Argument` has exactly one kind, set at construction time or via
/// the `toFlag()` / `toPositional()` builder methods. The default kind is
/// `.Option`.
///
/// # Examples
///
/// ```
/// let kind = ArgumentKind.Flag;
/// match kind {
///     .Flag => println("flag"),
///     .Option => println("option"),
///     .Positional => println("positional")
/// }
/// ```
public enum ArgumentKind: Cloneable {
    /// A boolean flag (`--verbose`, `-v`). Presence means `true`; no
    /// value is consumed from the token stream.
    case Flag

    /// A key-value option (`--output file`, `-o file`). The next token
    /// (or the text after `=`) is consumed as the value.
    case Option

    /// A positional argument, identified by order rather than by a flag
    /// prefix. Has no long or short flag.
    case Positional

    public func clone() -> ArgumentKind {
        match self {
            .Flag => .Flag,
            .Option => .Option,
            .Positional => .Positional
        }
    }
}

/// Defines a single CLI argument — a flag, option, or positional.
///
/// `Argument` uses a fluent builder pattern: every configuration method
/// returns a new copy with the requested change applied, so calls can be
/// chained in a single expression. The original value is never mutated.
///
/// New arguments default to `ArgumentKind.Option` with a long flag equal
/// to the name (e.g., `Argument("output")` responds to `--output`). Use
/// `toFlag()` or `toPositional()` to change the kind.
///
/// # Representation
///
/// Eight fields: a `name` and `kind`, optional long/short flags, optional
/// help text and placeholder, a required flag, and an optional default
/// value. All `Optional[String]` fields start as `.None`.
///
/// # Examples
///
/// ```
/// // A required positional argument
/// Argument("file")
///     .toPositional()
///     .required()
///     .help("Path to the input file")
///
/// // An option with a short alias and default
/// Argument("output")
///     .short("o")
///     .placeholder("FILE")
///     .help("Where to write the result")
///     .defaultsTo("out.txt")
///
/// // A boolean flag
/// Argument("verbose")
///     .short("v")
///     .toFlag()
///     .help("Enable verbose logging")
/// ```
public struct Argument: Cloneable {
    /// The argument's identifier, used to look up values in
    /// `ArgumentMatches`. Also serves as the default long flag for
    /// options (e.g., name `"output"` produces `--output`).
    public var name: String

    /// How this argument is parsed: flag, option, or positional.
    public var kind: ArgumentKind

    /// The long flag string, without the `--` prefix. `.None` for
    /// positional arguments.
    public var longFlag: Optional[String]

    /// The single-character short flag, without the `-` prefix.
    /// `.None` if no short alias is defined.
    public var shortFlag: Optional[String]

    /// Human-readable description shown in the help text.
    public var helpText: Optional[String]

    /// Placeholder name shown in the help column (e.g., `FILE` renders
    /// as `--output <FILE>`). Defaults to `VALUE` when absent.
    public var valueName: Optional[String]

    /// Whether the parser should reject input that omits this argument.
    public var isRequired: Bool

    /// Fallback value applied when the argument is absent from input
    /// and `isRequired` is `false`.
    public var defaultValue: Optional[String]

    /// @name Default
    /// Creates an option argument with the given name.
    ///
    /// The long flag is set to `name` automatically, so
    /// `Argument("output")` already responds to `--output`. Kind
    /// defaults to `.Option`; call `toFlag()` or `toPositional()` to
    /// change it.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("output");
    /// arg.name;      // "output"
    /// arg.isOption;  // true
    /// ```
    public init(name: String) {
        self.name = name;
        self.kind = ArgumentKind.Option;
        self.longFlag = .Some(name);
        self.shortFlag = .None;
        self.helpText = .None;
        self.valueName = .None;
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// @name Flag
    /// Creates a boolean flag argument.
    ///
    /// The long flag is set to `name`, so `Argument(flag: "verbose")`
    /// responds to `--verbose`. Flags are always optional.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument(flag: "verbose");
    /// arg.isFlag;  // true
    /// ```
    public init(flag name: String) {
        self.name = name;
        self.kind = ArgumentKind.Flag;
        self.longFlag = .Some(name);
        self.shortFlag = .None;
        self.helpText = .None;
        self.valueName = .None;
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// @name Flag with Description
    /// Creates a boolean flag with help text.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument(flag: "verbose", about: "Enable verbose output");
    /// arg.isFlag;    // true
    /// arg.helpText;  // .Some("Enable verbose output")
    /// ```
    public init(flag name: String, about about: String) {
        self.name = name;
        self.kind = ArgumentKind.Flag;
        self.longFlag = .Some(name);
        self.shortFlag = .None;
        self.helpText = .Some(about);
        self.valueName = .None;
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// @name Flag with Short Alias and Description
    /// Creates a boolean flag with a short alias and help text.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument(flag: "verbose", short: "v", about: "Enable verbose output");
    /// arg.isFlag;      // true
    /// arg.shortFlag;   // .Some("v")
    /// ```
    public init(flag name: String, short short: String, about about: String) {
        self.name = name;
        self.kind = ArgumentKind.Flag;
        self.longFlag = .Some(name);
        self.shortFlag = .Some(short);
        self.helpText = .Some(about);
        self.valueName = .None;
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// @name Positional
    /// Creates a required positional argument.
    ///
    /// Positionals are matched by order, not by flag prefix. Unlike
    /// options, they default to required.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument(positional: "file");
    /// arg.isPositional;  // true
    /// arg.isRequired;    // true
    /// ```
    public init(positional name: String) {
        self.name = name;
        self.kind = ArgumentKind.Positional;
        self.longFlag = .None;
        self.shortFlag = .None;
        self.helpText = .None;
        self.valueName = .None;
        self.isRequired = true;
        self.defaultValue = .None;
    }

    /// @name Positional with Description
    /// Creates a required positional argument with help text.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument(positional: "file", about: "Input file");
    /// arg.isPositional;  // true
    /// arg.isRequired;    // true
    /// ```
    public init(positional name: String, about about: String) {
        self.name = name;
        self.kind = ArgumentKind.Positional;
        self.longFlag = .None;
        self.shortFlag = .None;
        self.helpText = .Some(about);
        self.valueName = .None;
        self.isRequired = true;
        self.defaultValue = .None;
    }

    /// @name Option with Description
    /// Creates an option argument with help text.
    ///
    /// The long flag is set to `name` automatically.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("target", about: "Target triple");
    /// arg.isOption;   // true
    /// arg.helpText;   // .Some("Target triple")
    /// ```
    public init(name: String, about about: String) {
        self.name = name;
        self.kind = ArgumentKind.Option;
        self.longFlag = .Some(name);
        self.shortFlag = .None;
        self.helpText = .Some(about);
        self.valueName = .None;
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// @name Option with Short Alias and Description
    /// Creates an option with a short alias and help text.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("output", short: "o", about: "Output path");
    /// arg.shortFlag;  // .Some("o")
    /// ```
    public init(name: String, short short: String, about about: String) {
        self.name = name;
        self.kind = ArgumentKind.Option;
        self.longFlag = .Some(name);
        self.shortFlag = .Some(short);
        self.helpText = .Some(about);
        self.valueName = .None;
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// @name Option with Description and Placeholder
    /// Creates an option with help text and a placeholder name.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("target", about: "Target triple", placeholder: "TRIPLE");
    /// arg.valueName;  // .Some("TRIPLE")
    /// ```
    public init(name: String, about about: String, placeholder placeholder: String) {
        self.name = name;
        self.kind = ArgumentKind.Option;
        self.longFlag = .Some(name);
        self.shortFlag = .None;
        self.helpText = .Some(about);
        self.valueName = .Some(placeholder);
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// @name Option with Short Alias, Description, and Placeholder
    /// Creates a fully specified option argument.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("output", short: "o", about: "Output path", placeholder: "FILE");
    /// arg.shortFlag;  // .Some("o")
    /// arg.valueName;  // .Some("FILE")
    /// ```
    public init(name: String, short short: String, about about: String, placeholder placeholder: String) {
        self.name = name;
        self.kind = ArgumentKind.Option;
        self.longFlag = .Some(name);
        self.shortFlag = .Some(short);
        self.helpText = .Some(about);
        self.valueName = .Some(placeholder);
        self.isRequired = false;
        self.defaultValue = .None;
    }

    /// Creates a deep copy of every field.
    public func clone() -> Argument {
        var a = Argument(self.name.clone());
        a.kind = self.kind.clone();
        if let .Some(s) = self.longFlag { a.longFlag = .Some(s.clone()); } else { a.longFlag = .None; }
        if let .Some(s) = self.shortFlag { a.shortFlag = .Some(s.clone()); } else { a.shortFlag = .None; }
        if let .Some(s) = self.helpText { a.helpText = .Some(s.clone()); } else { a.helpText = .None; }
        if let .Some(s) = self.valueName { a.valueName = .Some(s.clone()); } else { a.valueName = .None; }
        a.isRequired = self.isRequired;
        if let .Some(s) = self.defaultValue { a.defaultValue = .Some(s.clone()); } else { a.defaultValue = .None; }
        a
    }

    // --- fluent builder methods ---

    /// Returns a copy with the given short flag character.
    ///
    /// The character is the single letter after `-` at the call site
    /// (e.g., `"v"` for `-v`). Combined short flags like `-vvv` and
    /// `-abc` are handled by the parser automatically.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("verbose").short("v");
    /// arg.shortFlag;  // .Some("v")
    /// ```
    public func short(shortFlag: String) -> Argument {
        var copy = self.clone();
        copy.shortFlag = .Some(shortFlag);
        copy
    }

    /// Returns a copy with the given long flag name.
    ///
    /// Overrides the default long flag (which equals the argument name).
    /// Pass the flag without the `--` prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("output").long("out");
    /// // responds to --out instead of --output
    /// ```
    public func long(longFlag: String) -> Argument {
        var copy = self.clone();
        copy.longFlag = .Some(longFlag);
        copy
    }

    /// Returns a copy with the given help description.
    ///
    /// The text appears in the right column of the auto-generated help
    /// output, aligned with other argument descriptions.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("output").help("Where to write the result");
    /// ```
    public func help(text: String) -> Argument {
        var copy = self.clone();
        copy.helpText = .Some(text);
        copy
    }

    /// Returns a copy with the given placeholder name for help output.
    ///
    /// Appears in angle brackets after the flag: `--output <FILE>`.
    /// When no placeholder is set the parser renders `<VALUE>`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("output").placeholder("FILE");
    /// // help shows: --output <FILE>    ...
    /// ```
    public func placeholder(name: String) -> Argument {
        var copy = self.clone();
        copy.valueName = .Some(name);
        copy
    }

    /// Returns a copy marked as required.
    ///
    /// If the argument is absent from the input and has no default
    /// value, parsing fails with `ParseError.MissingRequired`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("file").toPositional().required();
    /// arg.isRequired;  // true
    /// ```
    public func required() -> Argument {
        var copy = self.clone();
        copy.isRequired = true;
        copy
    }

    /// Returns a copy marked as optional (not required).
    ///
    /// Useful for overriding the default-required behavior of
    /// positionals created with `init(positional:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument(positional: "version").optional();
    /// arg.isRequired;  // false
    /// ```
    public func optional() -> Argument {
        var copy = self.clone();
        copy.isRequired = false;
        copy
    }

    /// Returns a copy marked as optional with a default value.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("jobs", about: "Parallel jobs").optional(defaultsTo: "4");
    /// arg.isRequired;    // false
    /// arg.defaultValue;  // .Some("4")
    /// ```
    public func optional(defaultsTo value: String) -> Argument {
        var copy = self.clone();
        copy.isRequired = false;
        copy.defaultValue = .Some(value);
        copy
    }

    /// Returns a copy with the given default value.
    ///
    /// Applied by the parser when the argument is absent from input.
    /// A default makes the argument effectively optional even if
    /// `required()` was not called.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("port").defaultsTo("8080");
    /// arg.defaultValue;  // .Some("8080")
    /// ```
    public func defaultsTo(value: String) -> Argument {
        var copy = self.clone();
        copy.defaultValue = .Some(value);
        copy
    }

    /// Returns a copy converted to a boolean flag.
    ///
    /// Flags consume no value from the token stream — their presence
    /// alone means `true`. The kind is set to `.Flag`; long and short
    /// flags are preserved so both `--verbose` and `-v` still work.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("verbose").short("v").toFlag();
    /// arg.isFlag;  // true
    /// ```
    public func toFlag() -> Argument {
        var copy = self.clone();
        copy.kind = ArgumentKind.Flag;
        copy
    }

    /// Returns a copy converted to a positional argument.
    ///
    /// Positional arguments are matched by order, not by flag prefix.
    /// Both long and short flags are cleared, since positionals have no
    /// `--`/`-` trigger.
    ///
    /// # Examples
    ///
    /// ```
    /// let arg = Argument("file").toPositional().required();
    /// arg.isPositional;  // true
    /// arg.longFlag;      // .None
    /// ```
    public func toPositional() -> Argument {
        var copy = self.clone();
        copy.kind = ArgumentKind.Positional;
        copy.longFlag = .None;
        copy.shortFlag = .None;
        copy
    }

    // --- queries ---

    /// `true` when this argument is a boolean flag.
    public var isFlag: Bool {
        match self.kind {
            .Flag => true,
            _ => false
        }
    }

    /// `true` when this argument is positional.
    public var isPositional: Bool {
        match self.kind {
            .Positional => true,
            _ => false
        }
    }

    /// `true` when this argument is a key-value option.
    public var isOption: Bool {
        match self.kind {
            .Option => true,
            _ => false
        }
    }
}
