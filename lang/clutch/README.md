# Clutch

CLI argument parsing for Kestrel.

## Installation

```toml
[dependencies]
kestrel/clutch = "0.1.0"
```

## Usage

Build CLI tools with a fluent API for defining commands, flags, and arguments.

```kestrel
let app = Command("myapp")
app.addArg(
    Arg("verbose")
        .short("v")
        .asFlag()
        .help("Enable verbose output")
)
app.addArg(
    Arg("output")
        .short("o")
        .placeholder("FILE")
        .help("Output file path")
)
app.addArg(
    Arg("input")
        .asPositional()
        .isRequired()
        .help("Input file")
)

match app.parse(args()) {
    .Ok(matches) => {
        if matches.hasFlag("verbose") {
            let _ = println("Verbose mode")
        }
        let output = matches.getValue("output")
        let input = matches.getValue("input")
    },
    .Err(e) => {
        let _ = eprintln(e.description())
    }
}
```

## Key Types

- **Command** - a CLI command with arguments and subcommands
- **Arg** - argument definition with flags, positional args, help text
- **ArgMatches** - result of parsing, holds matched values
- **ParseError** - parsing failure details

## Features

- Short and long flags (`-v`, `--verbose`)
- Required and optional arguments
- Positional arguments
- Auto-generated help text
- Subcommand support
