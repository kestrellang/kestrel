# Generics Implementation Plan

## Key Context

### SymbolId
Defined in `lib/semantic-tree/src/symbol/mod.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u64);

impl SymbolId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        SymbolId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
```
Use this for keys in Substitutions map.

### KestrelSymbolKind
In `lib/kestrel-semantic-tree/src/symbol/kind.rs` - needs `TypeParameter` variant:
```rust
pub enum KestrelSymbolKind {
    Field,
    Function,
    Import,
    Module,
    Protocol,
    SourceFile,
    Struct,
    TypeAlias,
    TypeParameter,  // ADD THIS
}
```

### Current TyKind
In `lib/kestrel-semantic-tree/src/ty/kind.rs`:
```rust
pub enum TyKind {
    Unit,
    Never,
    Int(IntBits),
    Float(FloatBits),
    Bool,
    String,
    Tuple(Vec<Ty>),
    Function { params: Vec<Ty>, return_type: Box<Ty> },
    Path(Vec<String>),
    Protocol(Arc<ProtocolSymbol>),
    Struct(Arc<StructSymbol>),
    TypeAlias(Arc<TypeAliasSymbol>),
}
```

### Ty struct helpers
`lib/kestrel-semantic-tree/src/ty/mod.rs` has constructor methods like `Ty::r#struct()`, `Ty::protocol()` etc. These need updating to accept Substitutions.

### Module exports
- `lib/kestrel-semantic-tree/src/ty/mod.rs` - add `pub mod substitutions; pub mod where_clause;`
- `lib/kestrel-semantic-tree/src/symbol/mod.rs` - add `pub mod type_parameter;`

### Resolver Pattern
From `lib/kestrel-semantic-tree-builder/src/builders/struct.rs`:
```rust
pub struct StructResolver;

impl Resolver for StructResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // 1. Extract name from syntax tree
        let name_str = extract_name(syntax)?;

        // 2. Extract visibility
        let visibility_behavior = ...;

        // 3. Create the symbol
        let struct_symbol = StructSymbol::new(name, full_span, visibility_behavior, parent.cloned());
        let struct_arc = Arc::new(struct_symbol);

        // 4. Add typed behavior
        let struct_type = Ty::r#struct(struct_arc.clone(), full_span);
        struct_arc.metadata().add_behavior(TypedBehavior::new(struct_type, full_span));

        // 5. Add to parent
        if let Some(parent) = parent {
            parent.metadata().add_child(&struct_arc_dyn);
        }

        Some(struct_arc)
    }
}
```

For generics, StructResolver needs to:
1. Look for TypeParameterList child node
2. Create TypeParameterSymbol for each
3. Look for WhereClause child node
4. Parse constraints
5. Pass type_parameters and where_clause to StructSymbol::new()

### Type Resolution Pattern
From `lib/kestrel-semantic-tree-builder/src/type_resolver.rs`:
```rust
pub fn resolve_type(ty: &Ty, ctx: &TypeResolutionContext, context_id: SymbolId) -> Option<Ty> {
    match ty.kind() {
        // Base types - return as-is
        TyKind::Unit | TyKind::Never | ... => Some(ty.clone()),

        // Path types - resolve via scope lookup
        TyKind::Path(segments) => {
            match queries::resolve_type_path(ctx.db, segments.clone(), context_id) {
                TypePathResolution::Resolved(resolved_ty) => Some(resolved_ty),
                _ => None,
            }
        }

        // Already resolved
        TyKind::Struct(_) | TyKind::Protocol(_) => Some(ty.clone()),

        // Recursive for composites
        TyKind::Tuple(elements) => { /* resolve each element */ }
        TyKind::Function { params, return_type } => { /* resolve params and return */ }
    }
}
```

For generics, resolve_type needs to:
1. Handle `TyKind::TypeParameter` - return as-is (already resolved)
2. Handle `TyKind::Struct { symbol, substitutions }` - recursively resolve types in substitutions
3. When resolving `TyKind::Path` that has type args in syntax, build Substitutions

### TypeResolutionContext Enhancement
Currently just holds `db`. May need to add:
```rust
pub struct TypeResolutionContext<'a> {
    pub db: &'a dyn Db,
    pub type_params_in_scope: HashMap<String, Arc<TypeParameterSymbol>>,  // for resolving T
}
```

---

## Design Decisions

### Syntax
- Type parameters: `[T, U, V]` (square brackets, not angle brackets)
- Defaults: `[T = Int]`
- Where clauses: `where T: Protocol and Protocol2, U: Other`
- Type arguments: `Foo[Int, String]`

### Variance
- Not exposed in syntax
- Always defaults to Invariant (the safest option)
- Stored on `TypeParameterSymbol` as a field set at creation time

### Where Clause Location
- Where clause lives on the container (Struct/Function/Protocol), not on individual type parameters
- Bounds are `Vec<Ty>` not `Vec<ProtocolSymbol>` to support generic bounds like `T: Iterator[Int]`

### Substitutions
- `Substitutions` type maps `SymbolId` → `Ty`
- Stored on instantiated types: `TyKind::Struct { symbol, substitutions }`
- Empty for non-generic types

---

## Data Structures

### TypeParameterSymbol
```rust
// lib/kestrel-semantic-tree/src/symbol/type_parameter.rs
pub struct TypeParameterSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    default: Option<Ty>,
    variance: Variance,
}

pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
    Bivariant,
}
```

### Substitutions
```rust
// lib/kestrel-semantic-tree/src/ty/substitutions.rs
pub struct Substitutions {
    map: HashMap<SymbolId, Ty>,
}

impl Substitutions {
    pub fn new() -> Self;
    pub fn is_empty(&self) -> bool;
    pub fn insert(&mut self, param_id: SymbolId, ty: Ty);
    pub fn get(&self, param_id: SymbolId) -> Option<&Ty>;
    pub fn apply(&self, ty: &Ty) -> Ty;  // substitute all type params
}
```

### WhereClause
```rust
// lib/kestrel-semantic-tree/src/ty/where_clause.rs
pub struct WhereClause {
    pub constraints: Vec<Constraint>,
}

pub enum Constraint {
    TypeBound {
        param: SymbolId,      // which type parameter
        bounds: Vec<Ty>,      // e.g., Iterator[Int], Equatable
    },
    // Future: TypeEquality for associated types
    // TypeEquality { left: TypePath, right: Ty }
}
```

### Updated TyKind
```rust
// lib/kestrel-semantic-tree/src/ty/kind.rs
pub enum TyKind {
    // Primitives (unchanged)
    Unit, Never, Int(IntBits), Float(FloatBits), Bool, String,

    // Composites (unchanged)
    Tuple(Vec<Ty>),
    Function { params: Vec<Ty>, return_type: Box<Ty> },

    // Path (unresolved)
    Path(Vec<String>),

    // NEW: Type parameter reference
    TypeParameter(Arc<TypeParameterSymbol>),

    // UPDATED: Instantiable types now carry substitutions
    Struct {
        symbol: Arc<StructSymbol>,
        substitutions: Substitutions,
    },
    Protocol {
        symbol: Arc<ProtocolSymbol>,
        substitutions: Substitutions,
    },
    TypeAlias {
        symbol: Arc<TypeAliasSymbol>,
        substitutions: Substitutions,
    },
}
```

### Updated Symbol Types
```rust
// StructSymbol
pub struct StructSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    type_parameters: Vec<Arc<TypeParameterSymbol>>,
    where_clause: WhereClause,
}

// FunctionSymbol
pub struct FunctionSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    type_parameters: Vec<Arc<TypeParameterSymbol>>,
    where_clause: WhereClause,
    // ... existing fields
}

// ProtocolSymbol
pub struct ProtocolSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    type_parameters: Vec<Arc<TypeParameterSymbol>>,
    where_clause: WhereClause,
}

// TypeAliasSymbol
pub struct TypeAliasSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    type_parameters: Vec<Arc<TypeParameterSymbol>>,
    where_clause: WhereClause,
}
```

---

## Implementation Progress

### Completed

#### Phase 2a: Lexer & Syntax Tree ✓
- Added `and` and `where` keywords to lexer (`lib/kestrel-lexer/src/lib.rs`)
- Added SyntaxKind variants (`lib/kestrel-syntax-tree/src/lib.rs`):
  - `TypeParameterList`, `TypeParameter`, `TypeArgumentList`
  - `DefaultType`, `WhereClause`, `TypeBound`, `TypeEquality`

#### Phase 2b: Parser - Type Parameters ✓
- Created `lib/kestrel-parser/src/type_param/mod.rs`
- Parsers: `type_parameter_list_parser()`, `where_clause_parser()`, `type_argument_list_parser()`
- Emit functions for syntax tree building
- 11 tests passing

#### Phase 2c: Parser - Type Updates ✓
- Updated `TyVariant` enum to support generic args
- Made type parser recursive for nested generics like `List[Option[Int]]`
- Added `emit_ty_variant()` central emitter
- 12 tests passing (including 3 generic type tests)

#### Phase 2d: Semantic Tree - Type Representation ✓
Files created/modified:
- Created `lib/kestrel-semantic-tree/src/ty/substitutions.rs` - Substitutions type with apply()
- Created `lib/kestrel-semantic-tree/src/ty/where_clause.rs` - WhereClause and Constraint types
- Created `lib/kestrel-semantic-tree/src/symbol/type_parameter.rs` - TypeParameterSymbol with Variance
- Modified `lib/kestrel-semantic-tree/src/ty/kind.rs` - TyKind now has TypeParameter and struct variants for Struct/Protocol/TypeAlias with substitutions
- Modified `lib/kestrel-semantic-tree/src/ty/mod.rs` - exports new modules
- Modified `lib/kestrel-semantic-tree/src/symbol/kind.rs` - added TypeParameter variant
- Modified `lib/kestrel-semantic-tree/src/behavior/callable.rs` - updated for new TyKind patterns

#### Phase 2e: Semantic Tree - Symbol Updates ✓
Files modified:
- `lib/kestrel-semantic-tree/src/symbol/struct.rs` - added type_parameters, where_clause, with_generics()
- `lib/kestrel-semantic-tree/src/symbol/function.rs` - added type_parameters, where_clause, with_generics()
- `lib/kestrel-semantic-tree/src/symbol/protocol.rs` - added type_parameters, where_clause, with_generics()
- `lib/kestrel-semantic-tree/src/symbol/type_alias.rs` - added type_parameters, where_clause, with_generics()
- `lib/kestrel-semantic-tree/src/symbol/mod.rs` - exports type_parameter module

#### Phase 2f: Semantic Tree Builder - Resolvers ✓
Files created/modified:
- Created `lib/kestrel-semantic-tree-builder/src/resolvers/type_parameter.rs` - extraction functions
- Modified `lib/kestrel-semantic-tree-builder/src/resolvers/struct.rs` - uses type parameter extraction
- Modified `lib/kestrel-semantic-tree-builder/src/resolvers/function.rs` - uses type parameter extraction
- Modified `lib/kestrel-semantic-tree-builder/src/resolvers/protocol.rs` - uses type parameter extraction
- Modified `lib/kestrel-semantic-tree-builder/src/resolvers/type_alias.rs` - uses type parameter extraction
- Modified `lib/kestrel-semantic-tree-builder/src/type_resolver.rs` - handles TypeParameter TyKind
- Modified `lib/kestrel-semantic-tree-builder/src/validation/visibility_consistency.rs` - handles new TyKind patterns
- Modified `lib/kestrel-semantic-tree-builder/src/lib.rs` - handles TypeParameter symbol kind

Key functions implemented:
- extract_type_parameters() - parses TypeParameterList from syntax
- extract_where_clause() - parses WhereClause from syntax
- build_type_param_map() - utility for name→SymbolId lookup

#### Phase 2g: Validation Passes ✓
Files created/modified:
- Created `lib/kestrel-semantic-tree/src/error.rs` - added error types:
  - `TypeArityError` - wrong number of type arguments
  - `TypeNotGenericError` - type args on non-generic type
  - `DuplicateTypeParameterError` - duplicate type param names
  - `DefaultOrderingError` - defaults must come after non-defaults
  - `NonProtocolBoundError` - bound is not a protocol
  - `UndeclaredTypeParameterError` - undeclared param in where clause
- Created `lib/kestrel-semantic-tree-builder/src/validation/generics.rs` - validation pass
- Modified `lib/kestrel-semantic-tree-builder/src/validation/mod.rs` - registered GenericsPass

Validations implemented:
- Duplicate type parameter names detection
- Default ordering check (defaults must come after non-defaults)

#### Phase 2h: Tests ✓
Files created/modified:
- Created `lib/kestrel-test-suite/tests/generics.rs` - comprehensive test file with:
  - basic_parsing module: struct/protocol/function/type_alias with generics
  - defaults module: type parameters with defaults
  - validation module: duplicate detection, default ordering
  - where_clause module: bounds testing
  - nested_generics module: generic types containing other generics
- Modified `lib/kestrel-test-suite/src/lib.rs` - added test helpers:
  - `Behavior::TypeParamCount(usize)` - check type parameter count
  - `Behavior::IsGeneric(bool)` - check if symbol is generic
  - `get_type_param_count()` helper function

Tests are distributed across:
- `lib/kestrel-test-suite/tests/types/generics.rs` - semantic/type tests
- `lib/kestrel-test-suite/tests/codegen/generics.rs` - codegen tests
- `lib/kestrel-test-suite/tests/execution_graph/generics.rs` - execution graph tests

#### Parser Integration ✓
Parser integration is complete:
- Updated `lib/kestrel-parser/src/struct/mod.rs` to parse TypeParameterList
- Updated `lib/kestrel-parser/src/function/mod.rs` to parse TypeParameterList
- Updated `lib/kestrel-parser/src/protocol/mod.rs` to parse TypeParameterList
- Updated `lib/kestrel-parser/src/type_alias/mod.rs` to parse TypeParameterList
- Updated `lib/kestrel-parser/src/enum/mod.rs` to parse TypeParameterList
- TypeParameterList and WhereClause nodes are emitted in syntax tree

---

## Error Messages

| Scenario | Error Message |
|----------|---------------|
| Too few args | "expected N type arguments, found M" |
| Too many args | "expected at most N type arguments, found M" |
| Args on non-generic | "type `X` does not take type arguments" |
| Unknown type param | "cannot find type `T` in this scope" |
| Duplicate param | "duplicate type parameter `T`" |
| Default ordering | "type parameter with default must come after parameters without defaults" |
| Non-protocol bound | "`X` is not a protocol" |
| Unknown bound | "cannot find protocol `X`" |
| Undeclared in where | "undeclared type parameter `T` in where clause" |

---

## Example Scenarios

### Basic Generic Struct
```
struct Box[T] {
    var value: T
}

let b: Box[Int]
```

### Multiple Type Parameters with Defaults
```
struct Map[K, V = String] { }

let a: Map[Int, Bool]   // K=Int, V=Bool
let b: Map[Int]         // K=Int, V=String (default)
```

### Where Clause with Generic Bound
```
struct Set[T] where T: Comparable[T] and Hashable { }
```

### Generic Function
```
func identity[T](value: T) -> T { }
```

### Nested Generics
```
struct Node[T] {
    var value: T
    var children: List[Node[T]]
}
```

---

## Critical: Type Extraction from Syntax

The `extract_field_type` function in `lib/kestrel-semantic-tree-builder/src/builders/field.rs` currently only extracts simple path types:

```rust
fn extract_field_type(syntax: &SyntaxNode, source: &str) -> Ty {
    // Finds Ty -> TyPath -> Path -> PathElement -> Identifier
    // Returns Ty::path(segments, span)
}
```

For generics, this needs to also look for `TypeArgumentList` children:

```
Ty
└── TyPath
    ├── Path
    │   └── PathElement
    │       └── Identifier ("List")
    └── TypeArgumentList        <-- NEW
        └── Ty
            └── TyPath
                └── Path...     ("Int")
```

The updated logic should:
1. Extract path segments as before
2. Check for TypeArgumentList child
3. If present, recursively extract type arguments
4. Return `Ty::path_with_args(segments, args, span)` or similar

This pattern repeats in function parameter/return type extraction too.

---

## Notes

- Pre-existing test failures in field/function/struct/type_alias parsers due to trivia handling (unrelated to generics)
- Type inference for generics is deferred to a later phase
- Associated types (`T.Item`) prepared for but not implemented
