# Modules

Modules are the primary organizational unit in Kestrel.
If a source file contains a `module` declaration, it contributes its declarations into that module path.

## Syntax

```
ModuleDeclaration → MODULE ModulePath

ModulePath → Identifier (DOT Identifier)*
```

### Tokens
- `MODULE` - The `module` keyword
- `DOT` - The `.` character
- `Identifier` - A valid identifier (Unicode XID_Start followed by XID_Continue*)

## Examples

```kestrel
// Simple module
module MyApp

// Nested module path
module MyApp.Core.Utils

// Unicode identifiers allowed
module café.αβγ
```

## Semantic Rules

### Rule 1: Module Declaration Optional

If a file has no module declaration, it is treated as belonging to the implicit root module.

```kestrel
// No module declaration: declarations attach to the root module
struct MyStruct { }
```

### Rule 2: Module Declaration Placement

If a file contains a module declaration, the first module declaration encountered in the syntax tree is used.
Additional module declarations (if any) are currently ignored during lowering.

```kestrel
import Lib

module App       // This module path is used
module Ignored   // Ignored during lowering
```

## Module Hierarchy

Module paths establish a hierarchy:

```kestrel
module A           // Root module A
module A.B         // Submodule B of A
module A.B.C       // Submodule C of A.B
```

Each segment creates a nested scope. The path `A.B.C` means:
- Module `C` is a child of module `B`
- Module `B` is a child of module `A`

## Module Scopes

Modules create scopes that contain:
- Imports (names from other modules)
- Declarations (structs, protocols, functions, type aliases, nested modules)

Declarations within a module are visible to:
- Other declarations in the same module (subject to visibility)
- Declarations in other modules (if visibility allows)

## Module Identity

Two modules are the same if and only if their full paths are identical:

```kestrel
// File 1
module A.B.C

// File 2
module A.B.C    // Same module as File 1

// File 3
module A.B      // Different module (parent of A.B.C)
```

Multiple files can declare the same module path, contributing declarations to the same logical module.

## Formal Semantics

Let `M` be a module declaration with path `p₁.p₂...pₙ`.

**ModuleSymbol Creation:**
1. For each segment `pᵢ`, ensure a ModuleSymbol exists
2. Establish parent-child relationships: `parent(pᵢ) = pᵢ₋₁`
3. The final segment `pₙ` is the declared module

**Scope Creation:**
```
scope(M) = {
    imports: ∅,           // Initially empty, populated by imports
    declarations: {...},   // Populated by declarations in file
    parent: scope(pₙ₋₁)   // Parent module's scope, or root if n=1
}
```

## Source Location

- **Build/lowering:** `lib/kestrel-semantic-tree-builder/src/lowerer.rs` (module path extraction + hierarchy)
- **Symbol type:** `ModuleSymbol` in `lib/kestrel-semantic-tree/src/symbol/module.rs`
