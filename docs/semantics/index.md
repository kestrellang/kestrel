# Kestrel Language Semantics

This documentation describes the formal semantics of the Kestrel programming language, including syntactic rules, semantic constraints, type resolution, and error conditions.

## Overview

Kestrel is a statically-typed language with:
- Module-based organization
- Four visibility levels (public, internal, fileprivate, private)
- Swift-style labeled function parameters
- Function overloading
- Protocols for interface definitions
- Structs for data types
- Type aliases
- Extensions for retroactive modeling
- Generics (type parameters, where clauses)
- Initializers

## Compilation Phases

### Phase 1: Lexing
Converts source text into tokens. See individual construct documentation for token definitions.

### Phase 2: Parsing
Converts tokens into a syntax tree. The parser uses error recovery to continue after syntax errors.

### Phase 3: Semantic Analysis (Build)
Lowering from syntax trees to an initial `SemanticModel`:
- Creates symbol hierarchy (modules, structs, functions, etc.)
- Stores syntax nodes and sources for later binding

### Phase 4: Semantic Analysis (Bind)
Resolves references and establishes relationships on a `SemanticModel`:
- Resolves type paths to concrete types
- Resolves conformances, extension targets, callable signatures, bodies
- Emits bind-time diagnostics (e.g. type resolution failures, duplicate signatures)

### Phase 5: Validation
Runs analyzers over the bound model (post-bind):
- Examples include: type alias cycle detection, duplicate symbols, visibility consistency,
  protocol method rules, type checking, imports validation.

## Documentation Index

### Core Constructs
- [Modules](modules.md) - Module declarations and organization
- [Imports](imports.md) - Import statements and module access
- [Types](types.md) - Type system overview
- [Type Aliases](type-aliases.md) - Type alias declarations

### Declarations
- [Functions](functions.md) - Function declarations and overloading
- [Structs](structs.md) - Struct declarations
- [Protocols](protocols.md) - Protocol declarations
- [Fields](fields.md) - Field declarations
- [Extensions](extensions.md) - Extension declarations
- [Initializers](initializers.md) - Initializer declarations
- [Generics](generics.md) - Type parameters and where clauses

### Resolution & Visibility
- [Visibility](visibility.md) - Access control system
- [Name Resolution](name-resolution.md) - How names are resolved
- [Type Resolution](type-resolution.md) - How types are resolved

### Reference
- [Errors](errors.md) - Complete error catalog

## Notation

Throughout this documentation:

- `→` denotes grammar production rules
- `|` denotes alternatives
- `?` denotes optional elements
- `*` denotes zero or more repetitions
- `+` denotes one or more repetitions
- `CAPS` denotes terminal tokens/keywords
- `lowercase` denotes non-terminal productions
- `"literal"` denotes literal syntax

Error conditions are shown as:

```
ERROR: ErrorName
WHEN: Condition that triggers the error
WHY: Explanation of why this is an error
```
