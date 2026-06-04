module clutch.command

import clutch.argument.(Argument)
import clutch.matches.(ArgumentMatches)
import clutch.error.(ParseError)
import clutch.parser.(parseCommand, CommandDefinition)
import clutch.help.(generateHelp)

/// A CLI command definition with arguments, flags, and subcommands.
///
/// `Command` is the root of a clutch CLI. Build one with the fluent API,
/// then call `parse(from:)` to turn a token array into
/// `ArgumentMatches`. Subcommands nest arbitrarily: each subcommand is
/// itself a `Command` with its own arguments and children.
///
/// The parser handles `--help` / `-h` automatically. When either flag
/// appears, `parse` short-circuits and returns
/// `ParseError.Message(helpText())` so you can print it through the
/// normal error path.
///
/// # Representation
///
/// Five fields: a `name`, optional `about` / `version` strings, and two
/// arrays — one for `Argument` definitions, one for child `Command`
/// subcommands. Builder methods clone-and-return so temporaries can be
/// chained.
///
/// # Examples
///
/// ```
/// let cmd = Command("mycli")
///     .about("A sample CLI tool")
///     .version("1.0.0")
///     .argument(Argument("verbose").short("v").toFlag().help("Enable verbose output"))
///     .argument(Argument("output").short("o").placeholder("FILE").help("Output file"))
///     .subcommand(
///         Command("build")
///             .about("Build the project")
///             .argument(Argument("target").toPositional().required().help("Build target"))
///     );
///
/// match cmd.parse(from: argv) {
///     .Ok(matches) => {
///         let verbose = matches.hasFlag("verbose");
///         let output = matches.value(for: "output");
///     },
///     .Err(e) => eprintln(e.description())
/// }
/// ```
public struct Command: Cloneable {
    /// The command name, shown in help output and usage lines.
    public var name: String

    /// Short description shown below the name in help output.
    var _about: Optional[String]

    /// Version string shown in the help header (e.g., `"1.0.0"`).
    var _version: Optional[String]

    /// Argument definitions registered with this command.
    var arguments: Array[Argument]

    /// Subcommand definitions. The first bare token that matches a
    /// subcommand name routes all remaining tokens to that child.
    var subcommands: Array[Command]

    /// @name Default
    /// Creates an empty command with the given name.
    ///
    /// The command starts with no arguments, no subcommands, and no
    /// about/version metadata. Use the fluent builder methods to
    /// configure it.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("flock");
    /// cmd.name;  // "flock"
    /// ```
    public init(name: String) {
        self.name = name;
        self._about = .None;
        self._version = .None;
        self.arguments = Array[Argument]();
        self.subcommands = Array[Command]();
    }

    /// @name With Description
    /// Creates a command with a name and description.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("flock", about: "Package manager");
    /// ```
    public init(name: String, about about: String) {
        self.name = name;
        self._about = .Some(about);
        self._version = .None;
        self.arguments = Array[Argument]();
        self.subcommands = Array[Command]();
    }

    /// @name With Description and Version
    /// Creates a command with a name, description, and version.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("flock", about: "Package manager", version: "0.3.0");
    /// ```
    public init(name: String, about about: String, version version: String) {
        self.name = name;
        self._about = .Some(about);
        self._version = .Some(version);
        self.arguments = Array[Argument]();
        self.subcommands = Array[Command]();
    }

    /// Creates a deep copy of the command and all its contents.
    public func clone() -> Command {
        var c = Command(self.name.clone());
        if let .Some(a) = self._about { c._about = .Some(a.clone()); }
        if let .Some(v) = self._version { c._version = .Some(v.clone()); }
        c.arguments = self.arguments.clone();
        c.subcommands = self.subcommands.clone();
        c
    }

    // --- fluent builder methods ---

    /// Returns a copy with the given description text.
    ///
    /// Appears on the second line of help output, directly below the
    /// name and version header.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("flock").about("Package manager for Kestrel");
    /// ```
    public func about(text: String) -> Command {
        var copy = self.clone();
        copy._about = .Some(text);
        copy
    }

    /// Returns a copy with the given version string.
    ///
    /// Shown next to the command name in the help header
    /// (e.g., `mycli 1.0.0`).
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli").version("1.0.0");
    /// ```
    public func version(versionString: String) -> Command {
        var copy = self.clone();
        copy._version = .Some(versionString);
        copy
    }

    /// Returns a copy with the given argument definition appended.
    ///
    /// Arguments are matched in registration order for positionals and
    /// by flag name for options/flags. Register all arguments before
    /// calling `parse`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument(Argument("verbose").short("v").toFlag())
    ///     .argument(Argument("output").short("o").help("Output path"));
    /// ```
    public func argument(argument: Argument) -> Command {
        var copy = self.clone();
        copy.arguments.append(argument);
        copy
    }

    /// Returns a copy with the given subcommand appended.
    ///
    /// During parsing, the first bare token that matches a subcommand
    /// name causes all remaining tokens to be parsed against that
    /// child's argument definitions. Access the matched subcommand name
    /// via `ArgumentMatches.subcommand` and its results via
    /// `ArgumentMatches.submatches`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("flock")
    ///     .subcommand(Command("build").about("Build the project"))
    ///     .subcommand(Command("run").about("Run the project"));
    /// ```
    public func subcommand(subcommand: Command) -> Command {
        var copy = self.clone();
        copy.subcommands.append(subcommand);
        copy
    }

    // --- with() ---

    /// Returns a copy with the given argument appended.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .with(argument: Argument(flag: "verbose", short: "v", about: "Be noisy"));
    /// ```
    public func with(argument argument: Argument) -> Command {
        var copy = self.clone();
        copy.arguments.append(argument);
        copy
    }

    /// Returns a copy with the given subcommand appended.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("flock")
    ///     .with(subcommand: Command("build", about: "Build the project"));
    /// ```
    public func with(subcommand subcommand: Command) -> Command {
        var copy = self.clone();
        copy.subcommands.append(subcommand);
        copy
    }

    // --- inline argument convenience methods ---

    /// Appends a flag argument inline.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument(flag: "release", about: "Build in release mode");
    /// ```
    public func argument(flag name: String, about about: String) -> Command {
        self.with(argument: Argument(flag: name, about: about))
    }

    /// Appends a flag argument with a short alias inline.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument(flag: "verbose", short: "v", about: "Enable verbose output");
    /// ```
    public func argument(flag name: String, short short: String, about about: String) -> Command {
        self.with(argument: Argument(flag: name, short: short, about: about))
    }

    /// Appends an option argument inline.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument("target", about: "Target triple");
    /// ```
    public func argument(name: String, about about: String) -> Command {
        self.with(argument: Argument(name, about: about))
    }

    /// Appends an option argument with a short alias inline.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument("output", short: "o", about: "Output path");
    /// ```
    public func argument(name: String, short short: String, about about: String) -> Command {
        self.with(argument: Argument(name, short: short, about: about))
    }

    /// Appends an option argument with a placeholder inline.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument("target", about: "Target triple", placeholder: "TRIPLE");
    /// ```
    public func argument(name: String, about about: String, placeholder placeholder: String) -> Command {
        self.with(argument: Argument(name, about: about, placeholder: placeholder))
    }

    /// Appends an option argument with short alias and placeholder.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument("output", short: "o", about: "Output path", placeholder: "FILE");
    /// ```
    public func argument(name: String, short short: String, about about: String, placeholder placeholder: String) -> Command {
        self.with(argument: Argument(name, short: short, about: about, placeholder: placeholder))
    }

    /// Appends a positional argument inline. Required by default.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument(positional: "file", about: "Input file");
    /// ```
    public func argument(positional name: String, about about: String) -> Command {
        self.with(argument: Argument(positional: name, about: about))
    }

    // --- parsing ---

    /// Parses an array of string tokens against this command.
    ///
    /// Walks the token array left-to-right, matching long flags
    /// (`--key`, `--key=value`), short flags (`-v`, `-vvv`, `-oValue`),
    /// the `--` separator (everything after is positional), subcommand
    /// names, and bare positional values.
    ///
    /// If `--help` or `-h` appears before the `--` separator, parsing
    /// short-circuits and returns `.Err(ParseError.Message(...))` with
    /// the formatted help text. This lets callers handle help through
    /// the same error path as real failures.
    ///
    /// After all tokens are consumed the parser applies default values
    /// for missing arguments and checks that every required argument
    /// was provided.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .argument(Argument("file").toPositional().required());
    ///
    /// match cmd.parse(from: ["hello.txt"]) {
    ///     .Ok(m) => m.value(of: "file"),  // .Some("hello.txt")
    ///     .Err(e) => eprintln(e.description())
    /// }
    /// ```
    public func parse(from tokens: Array[String]) -> Result[ArgumentMatches, ParseError] {
        if containsHelp(tokens) {
            return .Err(ParseError.Message(self.helpText()))
        }

        parseCommand(
            arguments: self.arguments,
            subcommands: self.buildSubcommandDefinitions(),
            tokens: tokens
        )
    }

    // --- help ---

    /// Builds the formatted help text for this command.
    ///
    /// The output includes a header (name + version), the about string,
    /// a `USAGE:` line, an `OPTIONS:` section for flags and options, an
    /// `ARGS:` section for positionals, and a `COMMANDS:` section for
    /// subcommands. A built-in `-h, --help` entry is always appended
    /// to the options list.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Command("mycli")
    ///     .about("A sample tool")
    ///     .argument(Argument("verbose").short("v").toFlag().help("Be noisy"));
    ///
    /// println(cmd.helpText());
    /// // mycli
    /// // A sample tool
    /// //
    /// // USAGE:
    /// //     mycli [OPTIONS]
    /// //
    /// // OPTIONS:
    /// //     -v, --verbose    Be noisy
    /// //     -h, --help       Print help
    /// ```
    public func helpText() -> String {
        var subNames = Array[String]();
        var subAbouts = Array[String]();

        for sub in self.subcommands {
            subNames.append(sub.name);
            match sub._about {
                .Some(a) => subAbouts.append(a),
                .None => subAbouts.append("")
            }
        }

        generateHelp(
            name: self.name,
            about: self._about,
            version: self._version,
            arguments: self.arguments,
            subcommandNames: subNames,
            subcommandAbouts: subAbouts
        )
    }

    // --- internal ---

    /// Flattens subcommands into `CommandDefinition` records for the
    /// recursive parser.
    func buildSubcommandDefinitions() -> Array[CommandDefinition] {
        var defs = Array[CommandDefinition]();
        for sub in self.subcommands {
            defs.append(CommandDefinition(
                name: sub.name,
                arguments: sub.arguments,
                subcommands: sub.buildSubcommandDefinitions()
            ));
        }
        defs
    }
}

/// Scans the token array for `--help` or `-h` before the `--` separator.
func containsHelp(tokens: Array[String]) -> Bool {
    for token in tokens {
        if token == "--help" or token == "-h" { return true }
        if token == "--" { return false }
    }
    false
}
