/// CLI argument parsing for Kestrel.
///
/// Clutch provides a fluent builder API for defining commands, flags,
/// options, and positional arguments, then parsing `argv` into a
/// structured `ArgumentMatches` result. Auto-generated help text is
/// built in.
///
/// # Examples
///
/// ```
/// import clutch.(Command, Argument, ArgumentMatches, ParseError)
/// import clutch.os.(getArgv)
///
/// let cmd = Command("mycli", about: "My CLI tool", version: "1.0.0")
///     .argument(flag: "verbose", short: "v", about: "Enable verbose output")
///     .argument("output", short: "o", about: "Output file path", placeholder: "FILE")
///     .argument(positional: "input", about: "Input file");
///
/// match cmd.parse(from: getArgv()) {
///     .Ok(matches) => {
///         let verbose = matches.hasFlag("verbose");
///         let output = matches.value(of: "output");
///         let input = matches.value(of: "input");
///     },
///     .Err(e) => {
///         eprintln(e.description())
///     }
/// }
/// ```

module clutch

import clutch.command.(Command)
import clutch.argument.(Argument, ArgumentKind)
import clutch.matches.(ArgumentMatches)
import clutch.error.(ParseError)
import clutch.os
