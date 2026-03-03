// Command definition and entry point

module clutch.command

import clutch.arg.(Arg)
import clutch.matches.(ArgMatches)
import clutch.error.(ParseError)
import clutch.parser.(parseCommand, CommandDef)
import clutch.help.(generateHelp)

// ============================================================================
// COMMAND
// ============================================================================

/// Defines a CLI command with arguments, flags, and subcommands.
///
/// Example:
///
///     var cmd = Command(name: "mycli")
///     cmd.setAbout(text: "A sample CLI tool")
///     cmd.setVersion(ver: "1.0.0")
///
///     var verbose = Arg(name: "verbose")
///     verbose.short(flag: "v")
///     verbose.asFlag()
///     verbose.help(text: "Enable verbose output")
///     cmd.addArg(arg: verbose)
///
///     match cmd.parse(tokens: args) {
///         .Ok(matches) => { ... },
///         .Err(e) => println(e.description())
///     }
///
public struct Command: Cloneable {
    public var name: String
    var about: Optional[String]
    var version: Optional[String]
    var args: Array[Arg]
    var subcommands: Array[Command]

    /// Creates a new command with the given name.
    public init(name name: String) {
        self.name = name;
        self.about = .None;
        self.version = .None;
        self.args = Array[Arg]();
        self.subcommands = Array[Command]();
    }

    public func clone() -> Command {
        var c = Command(name: self.name.clone());
        match self.about {
            .Some(a) => c.about = .Some(a.clone()),
            .None => {}
        }
        match self.version {
            .Some(v) => c.version = .Some(v.clone()),
            .None => {}
        }
        c.args = self.args.clone();
        c.subcommands = self.subcommands.clone();
        c
    }

    // --- builder methods ---

    /// Sets the about/description text for this command.
    public mutating func setAbout(text text: String) {
        self.about = .Some(text);
    }

    /// Sets the version string for this command.
    public mutating func setVersion(ver ver: String) {
        self.version = .Some(ver);
    }

    /// Adds an argument definition to this command.
    public mutating func addArg(arg arg: Arg) {
        self.args.append(arg)
    }

    /// Adds a subcommand to this command.
    public mutating func addSubcommand(sub sub: Command) {
        self.subcommands.append(sub)
    }

    // --- parsing ---

    /// Parses an array of string tokens against this command's definition.
    ///
    /// Handles --help/-h automatically by returning the help text as a
    /// ParseError.Message.
    public func parse(tokens tokens: Array[String]) -> Result[ArgMatches, ParseError] {
        // Check for --help / -h before parsing
        if containsHelp(tokens) {
            return .Err(ParseError.Message(self.helpText()))
        }

        parseCommand(
            args: self.args,
            subcommands: self.buildSubcommandDefs(),
            tokens: tokens
        )
    }

    // --- help ---

    /// Generates formatted help text for this command.
    public func helpText() -> String {
        var subNames = Array[String]();
        var subAbouts = Array[String]();

        var i: Int64 = 0;
        while i < self.subcommands.count {
            let sub = self.subcommands(unchecked: i);
            subNames.append(sub.name);
            match sub.about {
                .Some(a) => subAbouts.append(a),
                .None => subAbouts.append("")
            }
            i = i + 1
        }

        generateHelp(
            name: self.name,
            about: self.about,
            version: self.version,
            args: self.args,
            subcommandNames: subNames,
            subcommandAbouts: subAbouts
        )
    }

    // --- internal ---

    /// Converts subcommands to CommandDef for the parser.
    func buildSubcommandDefs() -> Array[CommandDef] {
        var defs = Array[CommandDef]();
        var i: Int64 = 0;
        while i < self.subcommands.count {
            let sub = self.subcommands(unchecked: i);
            defs.append(CommandDef(
                name: sub.name,
                args: sub.args,
                subcommands: sub.buildSubcommandDefs()
            ));
            i = i + 1
        }
        defs
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Checks if the tokens contain --help or -h at the top level.
func containsHelp(tokens: Array[String]) -> Bool {
    var i: Int64 = 0;
    while i < tokens.count {
        let token = tokens(unchecked: i);
        if token.equals("--help") or token.equals("-h") {
            return true
        }
        // Stop checking after -- separator
        if token.equals("--") {
            return false
        }
        i = i + 1
    }
    false
}
