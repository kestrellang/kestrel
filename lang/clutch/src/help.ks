// Help text generation

module clutch.help

import clutch.arg.(Arg)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Generates formatted help text for a command.
public func generateHelp(
    name name: String,
    about about: Optional[String],
    version version: Optional[String],
    args args: Array[Arg],
    subcommandNames subcommandNames: Array[String],
    subcommandAbouts subcommandAbouts: Array[String]
) -> String {
    var buf = String();

    // Header: name + version
    buf.append(name);
    match version {
        .Some(v) => {
            buf.append(" ");
            buf.append(v)
        },
        .None => {}
    }
    buf.append("\n");

    // About
    match about {
        .Some(a) => {
            buf.append(a);
            buf.append("\n")
        },
        .None => {}
    }

    buf.append("\n");

    // USAGE line
    buf.append("USAGE:");
    buf.append("\n");
    buf.append("    ");
    buf.append(name);

    // Check if there are any non-positional args
    var hasOptions = false;
    var i: Int64 = 0;
    while i < args.count {
        let arg = args(unchecked: i);
        if arg.isPositional() == false {
            hasOptions = true
        }
        i = i + 1
    }
    if hasOptions {
        buf.append(" [OPTIONS]")
    }

    // Append positional args to usage line
    i = 0;
    while i < args.count {
        let arg = args(unchecked: i);
        if arg.isPositional() {
            buf.append(" ");
            if arg.required {
                buf.append("<");
                buf.append(arg.name);
                buf.append(">")
            } else {
                buf.append("[");
                buf.append(arg.name);
                buf.append("]")
            }
        }
        i = i + 1
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

        // First pass: compute max left column width
        var maxLeft: Int64 = 0;
        i = 0;
        while i < args.count {
            let arg = args(unchecked: i);
            if arg.isPositional() == false {
                let width = leftColumnWidth(arg);
                if width > maxLeft {
                    maxLeft = width
                }
            }
            i = i + 1
        }

        // Add padding
        let padTo = maxLeft + 4;

        // Second pass: render each option
        i = 0;
        while i < args.count {
            let arg = args(unchecked: i);
            if arg.isPositional() == false {
                buf.append("    ");
                let left = formatLeftColumn(arg);
                buf.append(left);

                // Pad to alignment
                var pad = padTo - left.byteCount;
                while pad > 0 {
                    buf.append(" ");
                    pad = pad - 1
                }

                match arg.helpText {
                    .Some(h) => buf.append(h),
                    .None => {}
                }

                buf.append("\n")
            }
            i = i + 1
        }

        // Built-in help flag
        buf.append("    ");
        let helpLeft = "-h, --help";
        buf.append(helpLeft);
        var helpPad = padTo - helpLeft.byteCount;
        while helpPad > 0 {
            buf.append(" ");
            helpPad = helpPad - 1
        }
        buf.append("Print help");
        buf.append("\n");

        buf.append("\n")
    }

    // ARGS section (positionals)
    var hasPositionals = false;
    i = 0;
    while i < args.count {
        let arg = args(unchecked: i);
        if arg.isPositional() {
            hasPositionals = true
        }
        i = i + 1
    }

    if hasPositionals {
        buf.append("ARGS:");
        buf.append("\n");

        // Compute max name width
        var maxName: Int64 = 0;
        i = 0;
        while i < args.count {
            let arg = args(unchecked: i);
            if arg.isPositional() and arg.name.byteCount > maxName {
                maxName = arg.name.byteCount
            }
            i = i + 1
        }
        let namePadTo = maxName + 4;

        i = 0;
        while i < args.count {
            let arg = args(unchecked: i);
            if arg.isPositional() {
                buf.append("    ");
                if arg.required {
                    buf.append("<");
                    buf.append(arg.name);
                    buf.append(">");
                    var np = namePadTo - arg.name.byteCount - 2;
                    while np > 0 {
                        buf.append(" ");
                        np = np - 1
                    }
                } else {
                    buf.append("[");
                    buf.append(arg.name);
                    buf.append("]");
                    var np = namePadTo - arg.name.byteCount - 2;
                    while np > 0 {
                        buf.append(" ");
                        np = np - 1
                    }
                }

                match arg.helpText {
                    .Some(h) => buf.append(h),
                    .None => {}
                }

                buf.append("\n")
            }
            i = i + 1
        }

        buf.append("\n")
    }

    // SUBCOMMANDS section
    if subcommandNames.count > 0 {
        buf.append("COMMANDS:");
        buf.append("\n");

        // Compute max subcommand name width
        var maxSubName: Int64 = 0;
        i = 0;
        while i < subcommandNames.count {
            let sn = subcommandNames(unchecked: i);
            if sn.byteCount > maxSubName {
                maxSubName = sn.byteCount
            }
            i = i + 1
        }
        let subPadTo = maxSubName + 4;

        i = 0;
        while i < subcommandNames.count {
            buf.append("    ");
            let sn = subcommandNames(unchecked: i);
            buf.append(sn);

            var sp = subPadTo - sn.byteCount;
            while sp > 0 {
                buf.append(" ");
                sp = sp - 1
            }

            if i < subcommandAbouts.count {
                buf.append(subcommandAbouts(unchecked: i))
            }

            buf.append("\n");
            i = i + 1
        }

        buf.append("\n")
    }

    buf
}

// ============================================================================
// HELPERS
// ============================================================================

/// Computes the display width of the left column for an option.
func leftColumnWidth(arg: Arg) -> Int64 {
    var width: Int64 = 0;

    match arg.shortFlag {
        .Some(s) => {
            width = width + 1 + s.byteCount // "-v"
        },
        .None => {}
    }

    match arg.longFlag {
        .Some(l) => {
            match arg.shortFlag {
                .Some(_) => {
                    width = width + 2 // ", "
                },
                .None => {}
            }
            width = width + 2 + l.byteCount // "--verbose"
        },
        .None => {}
    }

    // Value placeholder for options
    if arg.isOption() {
        match arg.valueName {
            .Some(v) => {
                width = width + 2 + v.byteCount // " <FILE>"
            },
            .None => {
                width = width + 8 // " <VALUE>"
            }
        }
    }

    width
}

/// Formats the left column string for an option.
func formatLeftColumn(arg: Arg) -> String {
    var buf = String();

    match arg.shortFlag {
        .Some(s) => {
            buf.append("-");
            buf.append(s)
        },
        .None => {}
    }

    match arg.longFlag {
        .Some(l) => {
            match arg.shortFlag {
                .Some(_) => {
                    buf.append(", ")
                },
                .None => {}
            }
            buf.append("--");
            buf.append(l)
        },
        .None => {}
    }

    // Value placeholder for options
    if arg.isOption() {
        buf.append(" ");
        buf.append("<");
        match arg.valueName {
            .Some(v) => buf.append(v),
            .None => buf.append("VALUE")
        }
        buf.append(">")
    }

    buf
}
