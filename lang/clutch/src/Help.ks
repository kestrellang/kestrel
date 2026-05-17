/// Help-text generation for CLI commands.
///
/// Builds a multi-section string with USAGE, OPTIONS, ARGS, and
/// COMMANDS sections, column-aligned for terminal display. Called
/// internally by `Command.helpText()`; not typically used directly.

module clutch.help

import clutch.argument.(Argument)

/// Generates formatted help text for a command.
///
/// Assembles a header (name + version), about line, a `USAGE:` synopsis,
/// and up to three body sections — `OPTIONS:` (flags and key-value
/// options), `ARGS:` (positionals), and `COMMANDS:` (subcommands). Each
/// section is left-padded and column-aligned. A built-in `-h, --help`
/// entry is appended to the options list automatically.
///
/// # Examples
///
/// ```
/// let text = generateHelp(
///     name: "mycli",
///     about: .Some("A sample tool"),
///     version: .Some("1.0.0"),
///     arguments: [Argument("verbose").short("v").toFlag().help("Be noisy")],
///     subcommandNames: [],
///     subcommandAbouts: []
/// );
/// // mycli 1.0.0
/// // A sample tool
/// //
/// // USAGE:
/// //     mycli [OPTIONS]
/// //
/// // OPTIONS:
/// //     -v, --verbose    Be noisy
/// //     -h, --help       Print help
/// ```
public func generateHelp(
    name name: String,
    about about: Optional[String],
    version version: Optional[String],
    arguments arguments: Array[Argument],
    subcommandNames subcommandNames: Array[String],
    subcommandAbouts subcommandAbouts: Array[String]
) -> String {
    var buf = String();

    // Header: name + version
    buf.append(name);
    if let .Some(v) = version {
        buf.append(" ");
        buf.append(v);
    }
    buf.append("\n");

    // About
    if let .Some(a) = about {
        buf.append(a);
        buf.append("\n");
    }

    buf.append("\n");

    // USAGE line
    buf.append("USAGE:");
    buf.append("\n");
    buf.append("    ");
    buf.append(name);

    var hasOptions = false;
    for arg in arguments {
        if not arg.isPositional { hasOptions = true; }
    }
    if hasOptions {
        buf.append(" [OPTIONS]")
    }

    // Positional args on the usage line
    for arg in arguments {
        if not arg.isPositional { continue; }
        buf.append(" ");
        if arg.isRequired {
            buf.append("<");
            buf.append(arg.name);
            buf.append(">");
        } else {
            buf.append("[");
            buf.append(arg.name);
            buf.append("]");
        }
    }

    if subcommandNames.count > 0 {
        buf.append(" [COMMAND]")
    }

    buf.append("\n");
    buf.append("\n");

    // OPTIONS section
    if hasOptions {
        buf.append("OPTIONS:");
        buf.append("\n");

        var maxLeft: Int64 = 0;
        for arg in arguments {
            if arg.isPositional { continue; }
            let width = leftColumnWidth(arg);
            if width > maxLeft { maxLeft = width; }
        }

        let padTo = maxLeft + 4;

        for arg in arguments {
            if arg.isPositional { continue; }
            buf.append("    ");
            let left = formatLeftColumn(arg);
            buf.append(left);

            var pad = padTo - left.byteCount;
            while pad > 0 {
                buf.append(" ");
                pad = pad - 1;
            }

            if let .Some(h) = arg.helpText { buf.append(h); }

            buf.append("\n");
        }

        // Built-in help flag
        buf.append("    ");
        let helpLeft = "-h, --help";
        buf.append(helpLeft);
        var helpPad = padTo - helpLeft.byteCount;
        while helpPad > 0 {
            buf.append(" ");
            helpPad = helpPad - 1;
        }
        buf.append("Print help");
        buf.append("\n");

        buf.append("\n")
    }

    // ARGS section (positionals)
    var hasPositionals = false;
    for arg in arguments {
        if arg.isPositional { hasPositionals = true; }
    }

    if hasPositionals {
        buf.append("ARGS:");
        buf.append("\n");

        var maxName: Int64 = 0;
        for arg in arguments {
            if arg.isPositional and arg.name.byteCount > maxName {
                maxName = arg.name.byteCount;
            }
        }
        let namePadTo = maxName + 4;

        for arg in arguments {
            if not arg.isPositional { continue; }
            buf.append("    ");
            if arg.isRequired {
                buf.append("<");
                buf.append(arg.name);
                buf.append(">");
            } else {
                buf.append("[");
                buf.append(arg.name);
                buf.append("]");
            }

            var np = namePadTo - arg.name.byteCount - 2;
            while np > 0 {
                buf.append(" ");
                np = np - 1;
            }

            if let .Some(h) = arg.helpText { buf.append(h); }

            buf.append("\n");
        }

        buf.append("\n")
    }

    // COMMANDS section
    if subcommandNames.count > 0 {
        buf.append("COMMANDS:");
        buf.append("\n");

        var maxSubName: Int64 = 0;
        for sn in subcommandNames {
            if sn.byteCount > maxSubName { maxSubName = sn.byteCount; }
        }
        let subPadTo = maxSubName + 4;

        for i in 0..<subcommandNames.count {
            buf.append("    ");
            let sn = subcommandNames(unchecked: i);
            buf.append(sn);

            var sp = subPadTo - sn.byteCount;
            while sp > 0 {
                buf.append(" ");
                sp = sp - 1;
            }

            if i < subcommandAbouts.count {
                buf.append(subcommandAbouts(unchecked: i));
            }

            buf.append("\n");
        }

        buf.append("\n")
    }

    buf
}

/// Computes the display width of the left column for an option line.
///
/// Accounts for the short flag (`-v`), separator (`, `), long flag
/// (`--verbose`), and value placeholder (` <FILE>` or ` <VALUE>`).
func leftColumnWidth(argument: Argument) -> Int64 {
    var width: Int64 = 0;

    if let .Some(s) = argument.shortFlag {
        width = width + 1 + s.byteCount;
    }

    if let .Some(l) = argument.longFlag {
        if let .Some(_) = argument.shortFlag {
            width = width + 2;
        }
        width = width + 2 + l.byteCount;
    }

    if argument.isOption {
        match argument.valueName {
            .Some(v) => { width = width + 2 + v.byteCount; },
            .None => { width = width + 8; }
        }
    }

    width
}

/// Formats the left column string for an option line.
///
/// Produces strings like `-v, --verbose`, `--output <FILE>`, or
/// `-o, --output <VALUE>`.
func formatLeftColumn(argument: Argument) -> String {
    var buf = String();

    if let .Some(s) = argument.shortFlag {
        buf.append("-");
        buf.append(s);
    }

    if let .Some(l) = argument.longFlag {
        if let .Some(_) = argument.shortFlag {
            buf.append(", ");
        }
        buf.append("--");
        buf.append(l);
    }

    if argument.isOption {
        buf.append(" <");
        match argument.valueName {
            .Some(v) => buf.append(v),
            .None => buf.append("VALUE")
        }
        buf.append(">");
    }

    buf
}
