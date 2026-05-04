/// Core argument-parsing engine.
///
/// Implements the token-walking loop that resolves long flags, short
/// flags, the `--` separator, subcommand dispatch, and positional
/// arguments. Called internally by `Command.parse(from:)`; not
/// typically used directly.

module clutch.parser

import clutch.argument.(Argument, ArgumentKind)
import clutch.matches.(ArgumentMatches)
import clutch.error.(ParseError)

/// Parses a token array against a flat command definition.
///
/// Walks `tokens` left-to-right. Each token is classified as a long
/// flag (`--key` / `--key=value`), a short flag cluster (`-v`, `-vvv`,
/// `-oValue`), the `--` separator (everything after is positional), a
/// subcommand name, or a bare positional value.
///
/// After all tokens are consumed, default values are applied for
/// missing arguments and required-argument checks are enforced.
///
/// Returns `ArgumentMatches` on success or `ParseError` on the first
/// problem encountered.
public func parseCommand(
    arguments arguments: Array[Argument],
    subcommands subcommands: Array[CommandDefinition],
    tokens tokens: Array[String]
) -> Result[ArgumentMatches, ParseError] {
    var matches = ArgumentMatches();
    var pos: Int64 = 0;
    var positionalIndex: Int64 = 0;
    var seenDoubleDash = false;

    // pos is advanced non-linearly (consuming value tokens), so while is correct here
    while pos < tokens.count {
        let token = tokens(unchecked: pos);

        if seenDoubleDash {
            matches = try handlePositional(arguments, matches, token, positionalIndex);
            positionalIndex = positionalIndex + 1;
            pos = pos + 1;
            continue
        }

        if token == "--" {
            seenDoubleDash = true;
            pos = pos + 1;
            continue
        }

        // Long flags: --key or --key=value
        if token.starts(with: "--") {
            let rest = token.substringBytes(from: 2, to: token.byteCount);

            match rest.find("=") {
                .Some(eqPos) => {
                    let name = rest.substringBytes(from: 0, to: eqPos);
                    let value = rest.substringBytes(from: eqPos + 1, to: rest.byteCount);
                    let argDef = try findByLong(arguments, name);
                    if argDef.isFlag {
                        var msg = String();
                        msg.append("flag --");
                        msg.append(name);
                        msg.append(" does not accept a value");
                        return .Err(ParseError.Message(msg))
                    }
                    matches.setValue(name: argDef.name, value: value)
                },
                .None => {
                    let argDef = try findByLong(arguments, rest);
                    if argDef.isFlag {
                        matches.setFlag(name: argDef.name)
                    } else {
                        pos = pos + 1;
                        if pos >= tokens.count {
                            var msg = String();
                            msg.append("--");
                            msg.append(rest);
                            return .Err(ParseError.MissingValue(msg))
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
                let argDef = try findByShort(arguments, flagChar);

                if argDef.isFlag {
                    matches.setFlag(name: argDef.name);
                    charPos = charPos + 1
                } else {
                    if charPos + 1 < token.byteCount {
                        let value = token.substringBytes(from: charPos + 1, to: token.byteCount);
                        matches.setValue(name: argDef.name, value: value);
                        charPos = token.byteCount
                    } else {
                        pos = pos + 1;
                        if pos >= tokens.count {
                            var msg = String();
                            msg.append("-");
                            msg.append(flagChar);
                            return .Err(ParseError.MissingValue(msg))
                        }
                        matches.setValue(name: argDef.name, value: tokens(unchecked: pos));
                        charPos = token.byteCount
                    }
                }
            }

            pos = pos + 1;
            continue
        }

        // Bare token: subcommand match or positional
        if positionalIndex == 0 and subcommands.count > 0 {
            match findSubcommand(subcommands, token) {
                .Some(sub) => {
                    let remaining = sliceFrom(tokens, pos + 1);
                    let subMatches = try parseCommand(
                        arguments: sub.arguments,
                        subcommands: sub.subcommands,
                        tokens: remaining
                    );
                    matches.subcommand = .Some(token);
                    matches.submatches.append(subMatches);
                    return applyDefaultsAndCheck(arguments, matches)
                },
                .None => {
                    matches = try handlePositional(arguments, matches, token, positionalIndex);
                    positionalIndex = positionalIndex + 1
                }
            }
        } else {
            matches = try handlePositional(arguments, matches, token, positionalIndex);
            positionalIndex = positionalIndex + 1
        }

        pos = pos + 1
    }

    applyDefaultsAndCheck(arguments, matches)
}

/// Lightweight snapshot of a `Command` used during recursive parsing.
///
/// `Command` itself cannot be passed into `parseCommand` without a
/// circular dependency, so `Command.buildSubcommandDefinitions()`
/// flattens the tree into these records first.
public struct CommandDefinition: Cloneable {
    /// The subcommand name to match against bare tokens.
    public var name: String

    /// Argument definitions for this subcommand.
    public var arguments: Array[Argument]

    /// Nested subcommand definitions.
    public var subcommands: Array[CommandDefinition]

    public init(name name: String, arguments arguments: Array[Argument], subcommands subcommands: Array[CommandDefinition]) {
        self.name = name;
        self.arguments = arguments;
        self.subcommands = subcommands;
    }

    public func clone() -> CommandDefinition {
        CommandDefinition(name: self.name.clone(), arguments: self.arguments.clone(), subcommands: self.subcommands.clone())
    }
}

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

/// Looks up an argument definition by its long flag name.
func findByLong(arguments: Array[Argument], name: String) -> Result[Argument, ParseError] {
    for arg in arguments {
        guard let .Some(long) = arg.longFlag else { continue; }
        if long == name { return .Ok(arg) }
    }
    var msg = String();
    msg.append("--");
    msg.append(name);
    .Err(ParseError.UnknownFlag(msg))
}

/// Looks up an argument definition by its short flag character.
func findByShort(arguments: Array[Argument], flag: String) -> Result[Argument, ParseError] {
    for arg in arguments {
        guard let .Some(short) = arg.shortFlag else { continue; }
        if short == flag { return .Ok(arg) }
    }
    var msg = String();
    msg.append("-");
    msg.append(flag);
    .Err(ParseError.UnknownFlag(msg))
}

/// Looks up a subcommand definition by name.
func findSubcommand(definitions: Array[CommandDefinition], name: String) -> Optional[CommandDefinition] {
    for def in definitions {
        if def.name == name { return .Some(def) }
    }
    .None
}

/// Matches a bare token against the N-th positional argument slot.
func handlePositional(
    arguments: Array[Argument],
    mutating matches: ArgumentMatches,
    value: String,
    index: Int64
) -> Result[ArgumentMatches, ParseError] {
    var positionalCount: Int64 = 0;
    for arg in arguments {
        guard arg.isPositional else { continue; }
        if positionalCount == index {
            matches.setPositional(name: arg.name, value: value);
            return .Ok(matches)
        }
        positionalCount = positionalCount + 1;
    }
    .Err(ParseError.UnexpectedPositional(value))
}

/// Returns a tail slice of the array starting at `start`.
func sliceFrom(arr: Array[String], start: Int64) -> Array[String] {
    var result = Array[String]();
    for i in start..<arr.count {
        result.append(arr(unchecked: i));
    }
    result
}

/// Fills in default values for missing arguments and rejects input
/// that omits required arguments.
func applyDefaultsAndCheck(
    arguments: Array[Argument],
    mutating matches: ArgumentMatches
) -> Result[ArgumentMatches, ParseError] {
    for arg in arguments {
        guard not arg.isFlag else { continue; }

        if let .Some(_) = matches.value(for: arg.name) { continue; }

        if let .Some(def) = arg.defaultValue {
            if arg.isPositional {
                matches.setPositional(name: arg.name, value: def);
            } else {
                matches.setValue(name: arg.name, value: def);
            }
            continue;
        }

        if arg.isRequired {
            return .Err(ParseError.MissingRequired(arg.name))
        }
    }
    .Ok(matches)
}
