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
/// import clutch.command.(Command)
/// import clutch.argument.(Argument)
/// import clutch.matches.(ArgumentMatches)
/// import clutch.error.(ParseError)
/// import clutch.os.(getArgv)
///
/// let cmd = Command("mycli")
///     .about("My CLI tool")
///     .argument(Argument("verbose").short("v").toFlag().help("Enable verbose output"))
///     .argument(Argument("output").short("o").placeholder("FILE").help("Output file path"))
///     .argument(Argument("input").toPositional().required().help("Input file"));
///
/// match cmd.parse(from: getArgv()) {
///     .Ok(matches) => {
///         let verbose = matches.hasFlag("verbose");
///         let output = matches.value(for: "output");
///         let input = matches.value(for: "input");
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
