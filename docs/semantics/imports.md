# Imports

Imports bring names from other modules into the current scope, enabling access to external declarations.

## Syntax

```
ImportDeclaration → IMPORT ModulePath ImportTail?

ImportTail → ImportAlias
           | ImportItems

ImportAlias → AS Identifier

ImportItems → DOT LPAREN ImportItem (COMMA ImportItem)* COMMA? RPAREN

ImportItem → Identifier (AS Identifier)?
```

### Tokens
- `IMPORT` - The `import` keyword
- `AS` - The `as` keyword
- `DOT` - The `.` character
- `LPAREN` / `RPAREN` - Parentheses `(` `)`
- `COMMA` - The `,` character

## Import Forms

### 1. Whole-Module Import

```kestrel
import A.B.C
```

Imports ALL visible declarations from module `A.B.C` into the current scope. Each public/internal declaration in the module becomes available by its name.

**Effect:**
```
For each visible symbol S in module A.B.C:
    imports[S.name] = S
```

### 2. Module Alias Import

```kestrel
import A.B.C as X
```

Imports the module itself under the alias `X`. Access declarations via `X.DeclarationName`.

**Effect:**
```
imports["X"] = module(A.B.C)
```

### 3. Specific Item Import

```kestrel
import A.B.C.(Foo, Bar)
```

Imports only the specified items `Foo` and `Bar` from the module.

**Effect:**
```
imports["Foo"] = A.B.C.Foo
imports["Bar"] = A.B.C.Bar
```

### 4. Specific Item Import with Aliases

```kestrel
import A.B.C.(Foo as F, Bar as B)
```

Imports specific items under different names.

**Effect:**
```
imports["F"] = A.B.C.Foo
imports["B"] = A.B.C.Bar
```

### 5. Mixed Specific Items

```kestrel
import A.B.C.(Foo, Bar as B, Baz)
```

Some items with aliases, some without.

## Examples

```kestrel
module MyApp

// Import all from Utils
import MyApp.Utils

// Import module with alias
import ThirdParty.LongModuleName as Lib

// Import specific types
import MyApp.Models.(User, Account)

// Import with renaming to avoid conflicts
import OtherLib.Models.(User as OtherUser)

// Mixed
import MyApp.Core.(Config, Logger as Log, Constants)
```

## Semantic Rules

### Rule 1: Module Path Must Exist

The module path in an import must resolve to an existing module.

```
ERROR: ModuleNotFoundError
WHEN: A segment in the import path cannot be resolved
WHY: Cannot import from a non-existent module
```

**Error includes:**
- The partial path that was resolved
- The segment that failed
- Span of the failed segment

**Example:**
```kestrel
import NonExistent.Module    // ERROR: module 'NonExistent' not found
import Real.Fake.Module      // ERROR: module 'Real.Fake' not found (if Fake doesn't exist)
```

### Rule 2: Import Target Must Be a Module

The resolved path must point to a module, not a type or other declaration.

```
ERROR: CannotImportFromNonModuleError
WHEN: Import path resolves to a struct, protocol, or type alias
WHY: Only modules can contain importable declarations
```

**Example:**
```kestrel
module Other
public struct MyStruct { }

// In another file:
import Other.MyClass         // ERROR: cannot import from 'Other.MyClass': not a module
```

### Rule 3: Specific Items Must Exist

When importing specific items, each item must exist in the target module.

```
ERROR: SymbolNotFoundInModuleError
WHEN: A named item in the import list doesn't exist in the module
WHY: Cannot import a symbol that doesn't exist
```

**Example:**
```kestrel
// Module Utils only has: Logger, Config
import Utils.(Logger, Missing)    // ERROR: symbol 'Missing' not found in module 'Utils'
```

### Rule 4: Imported Items Must Be Visible

Imported symbols must have sufficient visibility to be accessed from the import site.

```
ERROR: SymbolNotVisibleError
WHEN: Attempting to import a symbol whose visibility doesn't allow access
WHY: Private/fileprivate symbols cannot be imported from outside their scope
```

**Visibility Requirements:**
- `public` - Can be imported from anywhere
- `internal` - Can be imported within the same top-level module
- `fileprivate` - Cannot be imported from other files
- `private` - Cannot be imported from outside the declaring scope

**Example:**
```kestrel
// In module A:
private struct Secret { }
public struct Public { }

// In module B:
import A.(Public)    // OK
import A.(Secret)    // ERROR: 'Secret' is not accessible (visibility: private)
```

### Rule 5: No Import Conflicts (Whole-Module Imports)

Whole-module imports cannot introduce names that conflict with existing declarations or other imports.

```
ERROR: ImportConflictError
WHEN: A whole-module import would shadow an existing name
WHY: Ambiguous names lead to confusion and errors
```

**Conflict sources:**
- Local declarations in the same file
- Names from previous imports

**Example:**
```kestrel
module MyApp

struct Logger { }           // Local declaration

import Utils               // Utils contains Logger
// ERROR: 'Logger' is already declared
```

**Resolution strategies:**
1. Use specific imports: `import Utils.(OtherThing)`
2. Use an alias: `import Utils as U` then `U.Logger`
3. Rename the local declaration

### Rule 6: Aliased Imports Never Conflict

Module aliases and specific item aliases never cause conflicts because they introduce a unique name.

```kestrel
import A.(Logger as ALogger)
import B.(Logger as BLogger)    // OK: different aliases
import C as CLib                // OK: module alias
```

## Import Resolution Process

### Phase 1: Module Path Resolution

```
resolve_module_path(path, context):
    current = root_scope
    for segment in path:
        current = find_child(current, segment)
        if current is None:
            return ModuleNotFoundError(segment)
        if current.kind != Module:
            return CannotImportFromNonModuleError(current.kind)
    return current
```

### Phase 2: Item Resolution (Specific Imports)

```
resolve_import_items(module, items):
    for item in items:
        symbol = find_in_module(module, item.name)
        if symbol is None:
            return SymbolNotFoundInModuleError(item.name)
        if not is_visible(symbol, import_context):
            return SymbolNotVisibleError(symbol)
        add_to_imports(item.alias or item.name, symbol)
```

### Phase 3: Conflict Detection (Whole-Module Imports)

```
check_whole_module_conflicts(module, current_scope):
    for symbol in visible_children(module):
        name = symbol.name
        if name in current_scope.declarations:
            return ImportConflictError(name, "declared")
        if name in current_scope.imports:
            return ImportConflictError(name, "imported")
        add_to_imports(name, symbol)
```

## Import Priority in Name Resolution

When resolving a name, imports are checked BEFORE local declarations:

```
resolve_name(name, scope):
    // 1. Check imports first (higher priority)
    if name in scope.imports:
        return scope.imports[name]

    // 2. Check local declarations
    if name in scope.declarations:
        return scope.declarations[name]

    // 3. Walk up to parent scope
    if scope.parent:
        return resolve_name(name, scope.parent)

    return NotFound
```

This means imports can shadow names from parent scopes, but NOT local declarations (which would be a conflict error for whole-module imports).

## Shadowing Rules

| Import Type | Can Shadow Parent Scope | Can Shadow Local Decl | Can Shadow Other Import |
|-------------|-------------------------|----------------------|-------------------------|
| Whole-module | Yes | No (error) | No (error) |
| Module alias | Yes | No (error) | No (error) |
| Specific item | Yes | No (error) | No (error) |
| Aliased item | Yes | Yes (via unique alias) | Yes (via unique alias) |

## Formal Semantics

Let `I` be an import declaration.

**Import Effect on Scope:**

For whole-module import `import M`:
```
∀ symbol S ∈ visible_children(M):
    if S.name ∈ scope.declarations ∪ scope.imports:
        error(ImportConflictError)
    else:
        scope.imports[S.name] = S
```

For aliased import `import M as A`:
```
scope.imports["A"] = M
```

For specific import `import M.(x₁, x₂, ..., xₙ)`:
```
∀ xᵢ ∈ {x₁, ..., xₙ}:
    S = lookup(M, xᵢ.name)
    if S is None:
        error(SymbolNotFoundInModuleError)
    if not visible(S, context):
        error(SymbolNotVisibleError)
    scope.imports[xᵢ.alias or xᵢ.name] = S
```

## Source Location

- **Build/lowering:** `lib/kestrel-semantic-tree-builder/src/builders/import.rs`
- **Bind:** `lib/kestrel-semantic-tree-binder/src/binders/import.rs`
- **Validate:** `lib/kestrel-semantic-analyzers/src/analyzers/imports/mod.rs`
- **Errors:** `lib/kestrel-semantic-tree/src/error.rs`
- **Symbol type:** `ImportSymbol` in `lib/kestrel-semantic-tree/src/symbol/import.rs`
- **Behavior:** `ImportDataBehavior` in `lib/kestrel-semantic-tree/src/behavior/import_data.rs`
