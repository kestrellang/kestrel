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
    .argument(Argument("verbose").short("v").toFlag().help("Enable verbose output"))
    .argument(Argument("output").short("o").placeholder("FILE").help("Output file path"))
    .argument(Argument("input").toPositional().required().help("Input file"));

match app.parse(from: args()) {
    .Ok(matches) => {
        if matches.hasFlag("verbose") {
            let _ = println("Verbose mode")
        }
        let output = matches.value(for: "output")
        let input = matches.value(for: "input")
    },
    .Err(e) => {
        let _ = eprintln(e.description())
    }
}
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
- Auto-generated help text
- Subcommand support
