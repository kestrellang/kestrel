// Clutch - CLI argument parsing for Kestrel
//
// Usage:
//
//     import clutch.command.(Command)
//     import clutch.arg.(Arg)
//     import clutch.matches.(ArgMatches)
//     import clutch.error.(ParseError)
//
//     var cmd = Command(name: "mycli")
//     cmd.setAbout(text: "My CLI tool")
//
//     var verbose = Arg(name: "verbose")
//     verbose.short(flag: "v")
//     verbose.asFlag()
//     verbose.help(text: "Enable verbose output")
//     cmd.addArg(arg: verbose)
//
//     var output = Arg(name: "output")
//     output.short(flag: "o")
//     output.placeholder(name: "FILE")
//     output.help(text: "Output file path")
//     cmd.addArg(arg: output)
//
//     var input = Arg(name: "input")
//     input.asPositional()
//     input.isRequired()
//     input.help(text: "Input file")
//     cmd.addArg(arg: input)
//
//     match cmd.parse(tokens: args) {
//         .Ok(matches) => {
//             let verbose = matches.hasFlag(name: "verbose")
//             let output = matches.getValue(name: "output")
//             let input = matches.getValue(name: "input")
//         },
//         .Err(e) => {
//             println(e.description())
//         }
//     }

module clutch

import clutch.command.(Command)
import clutch.arg.(Arg, ArgKind)
import clutch.matches.(ArgMatches)
import clutch.error.(ParseError)
import clutch.os
