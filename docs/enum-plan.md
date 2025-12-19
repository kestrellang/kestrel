# Enum Implementation Plan

Based on the design in [enums.md](./enums.md) and existing codebase patterns.

## Phase 1: Lexer

**File:** `lib/kestrel-lexer/src/lib.rs`

- Add `Enum` and `Case` as reserved keyword tokens
- `indirect` stays as identifier (contextual keyword, recognized by parser)

---

## Phase 2: Syntax Tree

**File:** `lib/kestrel-syntax-tree/src/lib.rs`

Add `SyntaxKind` variants:
- `EnumDeclaration`, `EnumBody`, `EnumCaseDeclaration`
- `EnumCaseParameter`, `EnumCaseParameterList`
- `IndirectModifier`

Update `From<Token>` and `kind_from_raw`.

---

## Phase 3: Parser

**New file:** `lib/kestrel-parser/src/enum/mod.rs`

**Pattern:** Follow `struct/mod.rs`

### Data Structures

Add to `common/data.rs`:

```rust
pub struct EnumDeclarationData {
    pub visibility: Option<(Token, Span)>,
    pub is_indirect: Option<Span>,
    pub enum_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub cases: Vec<EnumCaseData>,
    pub rbrace_span: Span,
}

pub struct EnumCaseData {
    pub case_span: Span,
    pub name_span: Span,
    pub parameters: Option<(Span, Vec<EnumCaseParameterData>, Span)>,
}

pub struct EnumCaseParameterData {
    pub label: Span,
    pub colon: Span,
    pub ty: TyVariant,
}
```

### Parser Logic

1. Check for `indirect` identifier before `enum`
2. Parse `enum Name[T] where T: Proto { ... }`
3. Parse cases: `case Name` or `case Name(label: Type, ...)`

Register in `declaration_item/mod.rs`.

---

## Phase 4: Semantic Symbols

**New files:**
- `lib/kestrel-semantic-tree/src/symbol/enum.rs` → `EnumSymbol`
- `lib/kestrel-semantic-tree/src/symbol/enum_case.rs` → `EnumCaseSymbol`

**Pattern:** Follow `struct.rs`

### Symbol Structure

```
EnumSymbol
├── is_indirect: bool
├── GenericsBehavior (type params + where clause)
├── TypedBehavior (TyKind::Enum)
└── children: Vec<EnumCaseSymbol>

EnumCaseSymbol
├── CallableBehavior (if has associated values)
└── parameters with labels and types
```

### KestrelSymbolKind

Update `lib/kestrel-semantic-tree/src/symbol/kind.rs`:

```rust
pub enum KestrelSymbolKind {
    // ... existing variants ...
    Enum,
    EnumCase,
}
```

---

## Phase 5: Type System

**File:** `lib/kestrel-semantic-tree/src/ty/kind.rs`

Add variant:

```rust
Enum {
    symbol: Arc<EnumSymbol>,
    substitutions: Substitutions,
}
```

**File:** `lib/kestrel-semantic-tree/src/ty/mod.rs`

Update methods:
- `is_assignable_to`
- `apply_substitutions`
- `substitute_self`
- `expand_aliases`
- Add `is_enum()`, `as_enum()`, `as_enum_with_subs()`

---

## Phase 6: Builder

**New files:**
- `lib/kestrel-semantic-tree-builder/src/builders/enum.rs`
- `lib/kestrel-semantic-tree-builder/src/builders/enum_case.rs`

**Pattern:** Follow `struct.rs`

### EnumBuilder

1. Extract name, visibility, indirect flag
2. Create `EnumSymbol`
3. Extract type parameters
4. Add `TypedBehavior` with `TyKind::Enum`
5. Add to parent

### EnumCaseBuilder

1. Extract name
2. Create `EnumCaseSymbol`
3. If has associated values, prepare for `CallableBehavior`
4. Add to parent enum

### Registration

**File:** `lib/kestrel-semantic-tree-builder/src/lowerer.rs`

```rust
SyntaxKind::EnumDeclaration => Some(&ENUM),
SyntaxKind::EnumCaseDeclaration => Some(&ENUM_CASE),
```

---

## Phase 7: Binder

**New files:**
- `lib/kestrel-semantic-tree-binder/src/binders/enum.rs`
- `lib/kestrel-semantic-tree-binder/src/binders/enum_case.rs`

### EnumBinder

1. Resolve generics (type parameters + where clause)
2. Validate all cases
3. Check for recursion (handled by analyzer)

### EnumCaseBinder

1. Resolve associated value types
2. Add `CallableBehavior` with resolved parameter types

### Registration

**File:** `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs`

```rust
binders.insert(SyntaxKind::EnumDeclaration, Box::new(EnumBinder));
binders.insert(SyntaxKind::EnumCaseDeclaration, Box::new(EnumCaseBinder));
```

---

## Phase 8: Body Resolver (Instantiation)

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs`

Handle instantiation expressions:

| Syntax | Resolution |
|--------|------------|
| `Color.Red` | Enum case value (no associated values) |
| `Shape.Circle(radius: 5.0)` | Call expression on enum case |
| `.Red` | Shorthand with bidirectional type inference |

### Full Path Resolution

- `ExprPath` ending with enum case after enum type
- Member access on enum type returning case value
- Call expressions for cases with associated values

### Shorthand Resolution

- `.Case` syntax requires expected type from context
- Use existing bidirectional type inference infrastructure

---

## Phase 9: Analyzers

**New file:** `lib/kestrel-semantic-analyzers/src/analyzers/recursive_enum/mod.rs`

### RecursiveEnumAnalyzer

```rust
pub struct RecursiveEnumAnalyzer;

impl Analyzer for RecursiveEnumAnalyzer {
    fn analyze(&mut self, model: &SemanticModel, ctx: &mut AnalysisContext) {
        // Walk all EnumSymbols
        // Check if any case references the enum type
        // Verify is_indirect is true if recursive
        // Emit E0404 if not
    }
}
```

### Additional Validations

- Duplicate case names (E0405)
- Duplicate labels in case (E0406)

---

## Phase 10: Error Codes

| Code | Error | Location |
|------|-------|----------|
| E0401 | Unknown enum case | Body resolver |
| E0402 | Missing/wrong label | Body resolver |
| E0403 | Cannot infer type for shorthand | Body resolver |
| E0404 | Recursive requires `indirect` | Analyzer |
| E0405 | Duplicate case name | Binder/Analyzer |
| E0406 | Duplicate label in case | Binder |
| E0407 | Type mismatch | Body resolver |
| E0408 | Wrong arity | Body resolver |

---

## Phase 11: Tests

**New file:** `lib/kestrel-test-suite/tests/declarations/enums.rs`

### Test Categories

1. Basic enum declaration
2. Enum with associated values (labeled)
3. Generic enums
4. Recursive/indirect enums
5. Enum instantiation (full path)
6. Enum instantiation (shorthand)
7. Error cases (all error codes)

### Test Framework Updates

**File:** `lib/kestrel-test-suite/src/lib.rs`

Add support for:
- `SymbolKind::Enum`
- `SymbolKind::EnumCase`

---

## Implementation Order

| Step | Phase | Description |
|------|-------|-------------|
| 1 | Lexer | Add `Enum` and `Case` tokens |
| 2 | Syntax Tree | Add `SyntaxKind` variants |
| 3 | Parser | Basic enum (no associated values) |
| 4 | Symbols | `EnumSymbol`, `EnumCaseSymbol` |
| 5 | Type System | `TyKind::Enum` |
| 6 | Builder | Basic enum building |
| 7 | Parser + Builder | Associated values with `CallableBehavior` |
| 8 | Binder | Type resolution |
| 9 | Body Resolver | Instantiation expressions |
| 10 | Analyzers | Validation (recursion, duplicates) |
| 11 | Tests | Comprehensive coverage |

---

## Key Reference Files

| Component | Reference Pattern |
|-----------|------------------|
| Parser | `lib/kestrel-parser/src/struct/mod.rs` |
| Symbol | `lib/kestrel-semantic-tree/src/symbol/struct.rs` |
| Builder | `lib/kestrel-semantic-tree-builder/src/builders/struct.rs` |
| Binder | `lib/kestrel-semantic-tree-binder/src/binders/struct.rs` |
| Type | `lib/kestrel-semantic-tree/src/ty/kind.rs` (Struct variant) |
