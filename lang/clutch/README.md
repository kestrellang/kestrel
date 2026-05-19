# Clutch

CLI argument parsing for Kestrel.

## Installation

```toml
[dependencies]
kestrel/clutch = "0.3.0"
```

## Usage

Build CLI tools with a fluent API for defining commands, flags, options,
and positional arguments.

```kestrel
import clutch.(Command, Argument, ArgumentMatches, ParseError)
import clutch.os.(getArgv)

let app = Command("myapp", about: "My CLI tool", version: "1.0.0")
    .argument(flag: "verbose", short: "v", about: "Enable verbose output")
    .argument("output", short: "o", about: "Output file path", placeholder: "FILE")
    .argument(positional: "input", about: "Input file");

match app.parse(from: getArgv()) {
    .Ok(matches) => {
        let verbose = matches.hasFlag("verbose");
        let output = matches.value(of: "output");
        let input = matches.value(of: "input");
    },
    .Err(e) => eprintln(e.description())
}
```

## Flags, Options, and Positionals

```kestrel
// Flags (boolean, always optional)
Argument(flag: "verbose")
Argument(flag: "verbose", short: "v", about: "Enable verbose output")

// Options (key-value, optional by default)
Argument("output", short: "o", about: "Output path", placeholder: "FILE")
Argument("jobs", about: "Parallel jobs").optional(defaultsTo: "4")

// Positionals (by order, required by default)
Argument(positional: "file", about: "Input file")
Argument(positional: "version", about: "Version constraint").optional()
```

## Subcommands

```kestrel
let cmd = Command("flock", about: "Package manager", version: "0.3.0")
    .with(subcommand:
        Command("build", about: "Compile the current package")
            .argument(flag: "release", short: "r", about: "Build in release mode")
            .argument("target", about: "Target triple", placeholder: "TRIPLE")
    )
    .with(subcommand:
        Command("run", about: "Run the package")
            .argument(positional: "args", about: "Arguments passed to the binary").optional()
    );
```

## Pre-built Arguments

Use `.with(argument:)` to add arguments constructed separately:

```kestrel
let verbose = Argument(flag: "verbose", short: "v", about: "Enable verbose output");
let output = Argument("output", short: "o", about: "Output path", placeholder: "FILE");

let cmd = Command("mycli")
    .with(argument: verbose)
    .with(argument: output);
```

## Key Types

- **Command** - a CLI command with arguments and subcommands
- **Argument** - argument definition with flags, positional args, help text
- **ArgumentMatches** - result of parsing, holds matched values
- **ParseError** - parsing failure details

## Features

- Short and long flags (`-v`, `--verbose`)
- Required and optional arguments
- Positional arguments
- Auto-generated help text (`--help`, `-h`)
- Subcommand support
- Default values
