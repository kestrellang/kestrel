# kestrel-compiler

A high-level compilation API for the Kestrel language, inspired by Roslyn's `Compilation` API.

## Overview

This crate provides an immutable builder pattern API for compiling Kestrel source files. It handles the entire compilation pipeline:

1. **Lexing** - Tokenization of source code
2. **Parsing** - Building syntax trees
3. **Semantic Analysis** - Building semantic trees with symbols
4. **Diagnostics** - Automatic collection of errors and warnings

## Features

- **Immutable builder pattern** - Clean, composable API
- **Multi-file support** - Compile multiple source files in one compilation
- **Automatic error collection** - Diagnostics from all phases collected automatically
- **Flexible input** - Add sources from strings or file paths
- **DiagnosticContext integration** - Full error reporting with `codespan-reporting`

## Usage

### Basic Example

```rust
use kestrel_compiler::Compilation;

// Create a compilation with source code
let compilation = Compilation::builder()
    .add_source("main.ks", "module Main\nclass Foo {}")
    .build();

// Check for errors
if compilation.has_errors() {
    compilation.diagnostics().emit().unwrap();
    std::process::exit(1);
}

// Access compiled results
for file in compilation.source_files() {
    println!("Compiled: {}", file.name());
}

// Access the unified semantic tree
if let Some(semantic_tree) = compilation.semantic_tree() {
    // Work with the semantic tree
    println!("Symbols: {}", semantic_tree.symbol_table().len());
}
```

### Multiple Files

```rust
let compilation = Compilation::builder()
    .add_source("main.ks", main_source)
    .add_source("utils.ks", utils_source)
    .add_file("config.ks")  // reads from disk
    .build();
```

### Accessing Compilation Results

```rust
// Get all source files
let files = compilation.source_files();

// Get a specific file
if let Some(file) = compilation.get_source_file("main.ks") {
    // Access source code
    let source = file.source();

    // Access syntax tree (per-file)
    let syntax_tree = file.syntax_tree();
}

// Access the unified semantic model (for entire compilation)
if let Some(model) = compilation.semantic_model() {
    let root = model.root();
}

// Work with diagnostics
let diagnostics = compilation.diagnostics();
println!("Total diagnostics: {}", diagnostics.len());
diagnostics.emit().unwrap();
```

## Architecture

The `Compilation` struct contains:
- **`source_files: Vec<SourceFile>`** - All compiled files
- **`semantic_model: Option<SemanticModel>`** - Unified semantic model for all files
- **`diagnostics: DiagnosticContext`** - All collected diagnostics

Each `SourceFile` contains:
- **`name: String`** - File name
- **`source: String`** - Original source code
- **`syntax_tree: SyntaxNode`** - Parsed syntax tree

The semantic model is stored at the compilation level to enable cross-file symbol resolution.

## Example

Run the example to see it in action:

```bash
cargo run --example basic_compilation -p kestrel-compiler
```

## API Similarity to Roslyn

This crate is inspired by Roslyn's C# Compilation API:

| Roslyn | Kestrel Compiler |
|--------|------------------|
| `CSharpCompilation.Create()` | `Compilation::builder()` |
| `.AddSyntaxTrees()` | `.add_source()` / `.add_file()` |
| `.GetDiagnostics()` | `.diagnostics()` |
| `SyntaxTree` | `SourceFile` |

## Integration with kestrel-reporting

The `DiagnosticContext` is automatically populated with:
- **Lexer errors** - Invalid tokens, with spans
- **Parser errors** - Syntax errors
- **Semantic errors** - Type errors, symbol resolution errors (coming soon)

All diagnostics can be emitted with colored, formatted output using `codespan-reporting`.
