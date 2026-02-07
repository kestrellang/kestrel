# Type Operators Implementation Plan

## Overview

This plan implements four type operators (`T?`, `[T]`, `[K: V]`, `T throws E`) as syntactic sugar that desugars to stdlib type aliases via the `@builtin` system.

**Major change:** Remove built-in `TyKind::Array` - arrays become regular generic structs.

## Test Strategy

### Test Categories
1. **Basic parsing** - Each type operator parses correctly
2. **Type resolution** - Operators resolve to correct underlying types
3. **Composability** - Operators compose with each other
4. **Error cases** - Invalid syntax produces helpful errors
5. **Integration** - Type operators work in all type positions (parameters, returns, fields, etc.)

### Key Behaviors to Verify
- `Int?` resolves to `Optional[Int]`
- `[Int]` resolves to `Array[Int]`
- `[String: Int]` resolves to `Dictionary[String, Int]`
- `Int throws Error` resolves to `Result[Int, Error]`
- `Int throws Error?` resolves to `Optional[Result[Int, Error]]` (precedence)
- Nested operators work: `[[Int?]]`, `[String: Int]?`, etc.

### Error Cases to Test
- `[]` - empty array type
- `[:]` - empty dictionary type
- `throws Error` - missing success type
- `Int throws` - missing error type

---

## Implementation Phases

### Phase 0: Tests (First!)

**Files:** `lib/kestrel-test-suite/tests/types/type_operators.rs`

- [ ] Create test file with `#[test]` functions
- [ ] Basic optional: `Int?` resolves correctly
- [ ] Basic array: `[Int]` resolves correctly
- [ ] Basic dictionary: `[String: Int]` resolves correctly
- [ ] Basic result: `Int throws Error` resolves correctly
- [ ] Precedence: `Int throws Error?` = `Optional[Result[Int, Error]]`
- [ ] Nested: `[[Int]]`, `[Int?]`, `[String: [Int]]`
- [ ] Complex: `[String: Int throws Error]?`
- [ ] Error: empty brackets, missing types

---

### Phase 1: Lexer

**Files:** `lib/kestrel-lexer/src/lib.rs`

- [ ] Add `Throws` token (~line 240, near other keywords)
  ```rust
  #[token("throws")]
  Throws,
  ```

---

### Phase 2: Syntax Tree

**Files:** `lib/kestrel-syntax-tree/src/lib.rs`

- [ ] Add `TyDictionary` variant to `SyntaxKind` enum (~line 113)
- [ ] Add `TyResult` variant to `SyntaxKind` enum
- [ ] Add `Throws` token variant if not already present
- [ ] Update `kind_from_raw()` match statement (~line 677+)

---

### Phase 3: Parser

**Files:** `lib/kestrel-parser/src/ty/mod.rs`

#### 3a: Dictionary type `[K: V]`

- [ ] Modify array parser to detect `:` after first type
- [ ] If colon found, parse as dictionary: `[K: V]`
- [ ] If no colon, parse as array: `[T]`
- [ ] Add `TyVariant::Dictionary(Span, Box<TyVariant>, Box<TyVariant>, Span)`
- [ ] Add `emit_dictionary_type()` function

#### 3b: Result type `T throws E`

- [ ] Add result type parsing after base type (similar to optional `?`)
- [ ] Parse: `base_type` then optionally `throws` then `error_type`
- [ ] `throws` should bind tighter than `?` (parse throws first, then check for `?`)
- [ ] Add `TyVariant::Result(Box<TyVariant>, Span, Box<TyVariant>)`
- [ ] Add `emit_result_type()` function

#### 3c: Precedence

Current parsing order for postfix/suffix:
1. Parse base type
2. Check for `?` (optional)

New order:
1. Parse base type
2. Check for `throws E` (result) - can repeat? No, one level only
3. Check for `?` (optional)

This gives `throws` higher precedence than `?`.

---

### Phase 4: Builtin Registry

**Files:** `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `LanguageFeature` variants:
  ```rust
  OptionalTypeOperator,
  ArrayTypeOperator,
  DictionaryTypeOperator,
  ResultTypeOperator,
  ```
- [ ] Add registration methods if needed (type aliases use `register_type_alias`)
- [ ] Add lookup methods: `type_alias(feature) -> Option<SymbolId>`

---

### Phase 5: Standard Library Type Aliases

**Files:**
- `lang/std/result/optional.ks`
- `lang/std/collections/array.ks`
- `lang/std/collections/dictionary.ks`
- `lang/std/result/result.ks`

- [ ] Add to `optional.ks`:
  ```kestrel
  @builtin(.OptionalTypeOperator)
  public type OptionalTypeOperator[T] = Optional[T]
  ```

- [ ] Add to `array.ks`:
  ```kestrel
  @builtin(.ArrayTypeOperator)
  public type ArrayTypeOperator[T] = Array[T]
  ```

- [ ] Add to `dictionary.ks`:
  ```kestrel
  @builtin(.DictionaryTypeOperator)
  public type DictionaryTypeOperator[K, V] = Dictionary[K, V]
  ```

- [ ] Add to `result.ks`:
  ```kestrel
  @builtin(.ResultTypeOperator)
  public type ResultTypeOperator[T, E] = Result[T, E]
  ```

---

### Phase 6: Type Resolver

**Files:** `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs`

#### 6a: Helper for builtin type alias lookup

- [ ] Add method to look up builtin type alias and apply type arguments:
  ```rust
  fn resolve_builtin_type_operator(
      &mut self,
      feature: LanguageFeature,
      type_args: Vec<Ty>,
      span: Span,
  ) -> Ty
  ```

#### 6b: Handle TyOptional (~line 141)

- [ ] Find `TyOptional` child node
- [ ] Extract base type from child `Ty` node
- [ ] Resolve base type recursively
- [ ] Look up `@builtin(.OptionalTypeOperator)`
- [ ] Apply `[base_ty]` as type argument
- [ ] Return resolved type

#### 6c: Handle TyArray (update existing, ~line 129)

- [ ] Keep existing parsing logic
- [ ] Change from `Ty::array(element_ty, ty_span)` to:
- [ ] Look up `@builtin(.ArrayTypeOperator)`
- [ ] Apply `[element_ty]` as type argument
- [ ] Return resolved type alias

#### 6d: Handle TyDictionary (new)

- [ ] Find `TyDictionary` child node
- [ ] Extract key type and value type from child `Ty` nodes
- [ ] Resolve both recursively
- [ ] Look up `@builtin(.DictionaryTypeOperator)`
- [ ] Apply `[key_ty, value_ty]` as type arguments
- [ ] Return resolved type

#### 6e: Handle TyResult (new)

- [ ] Find `TyResult` child node
- [ ] Extract success type and error type from child `Ty` nodes
- [ ] Resolve both recursively
- [ ] Look up `@builtin(.ResultTypeOperator)`
- [ ] Apply `[success_ty, error_ty]` as type arguments
- [ ] Return resolved type

---

### Phase 7: Remove TyKind::Array

**This is a large refactor. Do it last after everything else works.**

#### 7a: Remove from type system

**File:** `lib/kestrel-semantic-tree/src/ty/kind.rs`

- [ ] Remove `Array(Box<Ty>)` variant from `TyKind` enum
- [ ] Remove `Ty::array()` constructor method

#### 7b: Update expression resolver

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] Update array literal handling (~line 489-505)
- [ ] Instead of creating `TyKind::Array`, look up `Array` struct
- [ ] Create `TyKind::Struct { symbol: Array, substitutions: {T → element_ty} }`

#### 7c: Update all TyKind::Array references

Search for `TyKind::Array` and update each occurrence:

**Files to update (found via grep):**
- [ ] `lib/kestrel-semantic-tree/src/ty/kind.rs` - Display impl
- [ ] `lib/kestrel-semantic-tree/src/ty/substitutions.rs` - substitute()
- [ ] `lib/kestrel-semantic-tree/src/ty/normalize.rs` - normalize()
- [ ] `lib/kestrel-semantic-tree/src/expr.rs` - Expression::array()
- [ ] `lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs` - various
- [ ] `lib/kestrel-semantic-model/src/type_oracle.rs` - type checks
- [ ] `lib/kestrel-semantic-type-inference/src/apply.rs` - inference
- [ ] `lib/kestrel-semantic-type-inference/src/unify.rs` - unification
- [ ] `lib/kestrel-compiler/src/codegen/` - code generation

#### 7d: Update codegen

**Files:** `lib/kestrel-compiler/src/codegen/`

- [ ] Array operations must work with `Array` struct instead of primitive array type
- [ ] This may require significant codegen changes

---

### Phase 8: Documentation

- [ ] Update `docs/language/types.md` with type operator syntax
- [ ] Update `docs/ai-kestrel-guide.md` with examples
- [ ] Mark completed in `TODO.md` and `ROADMAP.md`

---

## Verification

After each phase:
```bash
cargo test
cargo clippy
cargo fmt --check
```

Final verification:
```bash
cargo test -p kestrel-test-suite
```

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Removing TyKind::Array breaks many things | Do it last, after all other phases work |
| Builtin lookup fails at runtime | Add clear error messages, test thoroughly |
| Parser ambiguity with `[T]` vs `[K: V]` | Lookahead for `:` after first type |
| Precedence confusion with `throws` | Document clearly, test edge cases |

---

## Estimated Scope

- **Lexer:** ~5 lines
- **Syntax tree:** ~10 lines
- **Parser:** ~100 lines
- **Builtin registry:** ~20 lines
- **Stdlib aliases:** ~20 lines
- **Type resolver:** ~100 lines
- **Remove TyKind::Array:** ~500+ lines across many files (DEFERRED)

## Scope Decision

**Phase 7 (Remove TyKind::Array) is deferred to a separate follow-up task.**

This PR implements Phases 0-6:
- Type operators will work alongside the existing `TyKind::Array`
- `[T]` syntax will resolve via `ArrayTypeOperator` → `Array[T]` struct
- The built-in `TyKind::Array` remains but is not used by the new syntax
- A follow-up PR will remove `TyKind::Array` entirely

## Implementation Status

**Type alias normalization has been implemented.** The following type operators now work:

| Operator | Status | Notes |
|----------|--------|-------|
| `T?` | Working | Desugars to `Optional[T]` via `OptionalTypeOperator` |
| `[K: V]` | Working | Desugars to `Dictionary[K, V]` via `DictionaryTypeOperator` |
| `T throws E` | Working | Desugars to `Result[T, E]` via `ResultTypeOperator` |
| `[T]` | Deferred | Still uses built-in `TyKind::Array` (see below) |

### Array Type Operator Deferred

The `[T]` syntax currently uses the built-in `TyKind::Array` type instead of `ArrayTypeOperator[T]`.
This is because `TyKind::Array` is deeply integrated into:
- Array pattern matching (`[first, ..rest]`)
- Array literal type inference (`[1, 2, 3]`)
- Other array-specific compiler features

Switching to `ArrayTypeOperator` requires the full Phase 7 work (removing `TyKind::Array` from the type system).
