// Core argument parsing logic

module clutch.parser

import clutch.arg.(Arg, ArgKind)
import clutch.matches.(ArgMatches)
import clutch.error.(ParseError)

// ============================================================================
// PUBLIC API
// ============================================================================

/// Parses an array of string arguments against a command definition.
///
/// The command is defined by its args, subcommands, and metadata.
/// Returns ArgMatches on success, or ParseError on failure.
public func parseCommand(
    args args: Array[Arg],
    subcommands subcommands: Array[CommandDef],
    tokens tokens: Array[String]
) -> Result[ArgMatches, ParseError] {
    var matches = ArgMatches();
    var pos: Int64 = 0;
    var positionalIndex: Int64 = 0;
    var seenDoubleDash = false;

    while pos < tokens.count {
        let token = tokens(unchecked: pos);

        if seenDoubleDash {
            // Everything after -- is positional
            matches = try handlePositional(args, matches, token, positionalIndex);
            positionalIndex = positionalIndex + 1;
            pos = pos + 1;
            continue
        }

        // -- separator
        if token.equals("--") {
            seenDoubleDash = true;
            pos = pos + 1;
            continue
        }

        // Long flags: --key or --key=value
        if token.starts(with: "--") {
            let rest = token.substringBytes(from: 2, to: token.byteCount);

            // Check for --key=value syntax
            match rest.find("=") {
                .Some(eqPos) => {
                    let name = rest.substringBytes(from: 0, to: eqPos);
                    let value = rest.substringBytes(from: eqPos + 1, to: rest.byteCount);
                    let argDef = try findByLong(args, name);
                    if argDef.isFlag() {
                        return .Err(ParseError.Message("flag --" + name + " does not accept a value"))
                    }
                    matches.setValue(name: argDef.name, value: value)
                },
                .None => {
                    let argDef = try findByLong(args, rest);
                    if argDef.isFlag() {
                        matches.setFlag(name: argDef.name)
                    } else {
                        // Consume next token as value
                        pos = pos + 1;
                        if pos >= tokens.count {
                            return .Err(ParseError.MissingValue("--" + rest))
                        }
                        matches.setValue(name: argDef.name, value: tokens(unchecked: pos))
                    }
                }
            }

            pos = pos + 1;
            continue
        }

        // Short flags: -v, -vvv, -abc, -o value, -ovalue
        if token.starts(with: "-") and token.byteCount > 1 {
            var charPos: Int64 = 1;
            while charPos < token.byteCount {
                let flagChar = token.substringBytes(from: charPos, to: charPos + 1);
                let argDef = try findByShort(args, flagChar);

                if argDef.isFlag() {
                    matches.setFlag(name: argDef.name);
                    charPos = charPos + 1
                } else {
                    // Short option: rest of token or next token is the value
                    if charPos + 1 < token.byteCount {
                        // -oValue: value is the rest of this token
                        let value = token.substringBytes(from: charPos + 1, to: token.byteCount);
                        matches.setValue(name: argDef.name, value: value);
                        charPos = token.byteCount
                    } else {
                        // -o Value: value is next token
                        pos = pos + 1;
                        if pos >= tokens.count {
                            return .Err(ParseError.MissingValue("-" + flagChar))
                        }
                        matches.setValue(name: argDef.name, value: tokens(unchecked: pos));
                        charPos = token.byteCount
                    }
                }
            }

            pos = pos + 1;
            continue
        }

        // Bare token: check for subcommand first, then positional
        if positionalIndex == 0 and subcommands.count > 0 {
            match findSubcommand(subcommands, token) {
                .Some(sub) => {
                    // Parse remaining tokens against the subcommand
                    let remaining = sliceFrom(tokens, pos + 1);
                    let subMatches = try parseCommand(
                        args: sub.args,
                        subcommands: sub.subcommands,
                        tokens: remaining
                    );
                    matches.subcommand = .Some(token);
                    matches.submatches.append(subMatches);
                    // Subcommand consumes all remaining tokens
                    return applyDefaultsAndCheck(args, matches)
                },
                .None => {
                    // Not a subcommand, treat as positional
                    matches = try handlePositional(args, matches, token, positionalIndex);
                    positionalIndex = positionalIndex + 1
                }
            }
        } else {
            matches = try handlePositional(args, matches, token, positionalIndex);
            positionalIndex = positionalIndex + 1
        }

        pos = pos + 1
    }

    applyDefaultsAndCheck(args, matches)
}

// ============================================================================
// SUBCOMMAND DEFINITION
// ============================================================================

/// Lightweight command definition for subcommand matching.
public struct CommandDef: Cloneable {
    public var name: String
    public var args: Array[Arg]
    public var subcommands: Array[CommandDef]

    public init(name name: String, args args: Array[Arg], subcommands subcommands: Array[CommandDef]) {
        self.name = name;
        self.args = args;
        self.subcommands = subcommands;
    }

    public func clone() -> CommandDef {
        CommandDef(name: self.name.clone(), args: self.args.clone(), subcommands: self.subcommands.clone())
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Finds an arg definition by its long flag name.
func findByLong(args: Array[Arg], name: String) -> Result[Arg, ParseError] {
    var i: Int64 = 0;
    while i < args.count {
        let arg = args(unchecked: i);
        match arg.longFlag {
            .Some(long) => {
                if long.equals(name) {
                    return .Ok(arg)
                }
            },
            .None => {}
        }
        i = i + 1
    }
    .Err(ParseError.UnknownFlag("--" + name))
}

/// Finds an arg definition by its short flag character.
func findByShort(args: Array[Arg], flag: String) -> Result[Arg, ParseError] {
    var i: Int64 = 0;
    while i < args.count {
        let arg = args(unchecked: i);
        match arg.shortFlag {
            .Some(short) => {
                if short.equals(flag) {
                    return .Ok(arg)
                }
            },
            .None => {}
        }
        i = i + 1
    }
    .Err(ParseError.UnknownFlag("-" + flag))
}

/// Finds a subcommand by name.
func findSubcommand(subs: Array[CommandDef], name: String) -> Optional[CommandDef] {
    var i: Int64 = 0;
    while i < subs.count {
        let sub = subs(unchecked: i);
        if sub.name.equals(name) {
            return .Some(sub)
        }
        i = i + 1
    }
    .None
}

/// Handles a positional argument by finding the N-th positional arg definition.
func handlePositional(
    args: Array[Arg],
    mutating matches: ArgMatches,
    value: String,
    index: Int64
) -> Result[ArgMatches, ParseError] {
    var positionalCount: Int64 = 0;
    var i: Int64 = 0;
    while i < args.count {
        let arg = args(unchecked: i);
        if arg.isPositional() {
            if positionalCount == index {
                matches.setPositional(name: arg.name, value: value);
                return .Ok(matches)
            }
            positionalCount = positionalCount + 1
        }
        i = i + 1
    }
    .Err(ParseError.UnexpectedPositional(value))
}

/// Creates a new array from elements starting at the given index.
func sliceFrom(arr: Array[String], start: Int64) -> Array[String] {
    var result = Array[String]();
    var i = start;
    while i < arr.count {
        result.append(arr(unchecked: i));
        i = i + 1
    }
    result
}

/// Applies default values and checks required args.
func applyDefaultsAndCheck(
    args: Array[Arg],
    mutating matches: ArgMatches
) -> Result[ArgMatches, ParseError] {
    var i: Int64 = 0;
    while i < args.count {
        let arg = args(unchecked: i);

        // Skip flags
        if arg.isFlag() {
            i = i + 1;
            continue
        }

        // Check if value was provided
        let hasValue = match matches.getValue(name: arg.name) {
            .Some(_) => true,
            .None => false
        };

        if hasValue == false {
            // Apply default if available
            match arg.defaultValue {
                .Some(def) => {
                    if arg.isPositional() {
                        matches.setPositional(name: arg.name, value: def)
                    } else {
                        matches.setValue(name: arg.name, value: def)
                    }
                },
                .None => {
                    // Check required
                    if arg.required {
                        return .Err(ParseError.MissingRequired(arg.name))
                    }
                }
            }
        }

        i = i + 1
    }
    .Ok(matches)
}
