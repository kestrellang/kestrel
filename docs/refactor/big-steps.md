# Phase 15: Big Steps Refactoring Plan

Three major refactoring efforts that address the biggest architectural pain points in the compiler.

---

## Table of Contents

- [1. Move All Resolution to Inference](#1-move-all-resolution-to-inference)
- [2. Unified Type Transformation Pipeline](#2-unified-type-transformation-pipeline)
- [3. Parser Rewrite: Hand-Written Recursive Descent](#3-parser-rewrite-hand-written-recursive-descent)
- [4. Symbol Mangling Rewrite](#4-symbol-mangling-rewrite)
- [Implementation Order](#implementation-order)

---

## 1. Move All Resolution to Inference

### Problem

There are two separate code paths for resolving method calls and member access:

**Path A — Binder (BIND phase):** `body_resolver/calls.rs` (3156 lines) + `body_resolver/members.rs` (3146 lines) + `body_resolver/paths.rs` (1620 lines) = ~8000 lines of resolution logic. This eagerly resolves method calls, member access, overloads, extensions, and protocol bounds before type inference runs.

**Path B — TypeOracle (VALIDATE/inference phase):** `type_oracle.rs` (4181 lines) implementing `TypeOracle::resolve_member()`. This handles deferred method calls and constraint-based resolution during type inference.

Both paths duplicate:
- Receiver type expansion and alias handling
- Substitution application (`apply_substitutions` → `substitute_self` → `resolve_associated_types`)
- Protocol bound traversal and extension search
- Overload matching and visibility checking
- Where clause constraint checking at call sites

The `DeferredMethodCall` mechanism exists because some calls *can't* be resolved eagerly — the type information isn't available yet during BIND. Every time a case is discovered where eager resolution gets the wrong answer, a new deferred path has to be added. This is evidence that eager resolution is fighting the design.

### Solution

Make `DeferredMethodCall` the *only* path, not the exception. During BIND, emit unresolved expressions. During inference, resolve everything.

#### What BIND still does (lightweight)

- Resolve imports and type references (needed to build the type graph)
- Resolve simple variable references (`x` → local, field, or function)
- Build expression tree structure (if/while/match/closures)
- Create locals with type annotations
- Resolve type syntax (`Array[Int]`, `(Int) -> String`, etc.)

#### What BIND stops doing

Everything currently in `members.rs`, most of `calls.rs`, and chunks of `paths.rs`:

- Method/member lookup on receiver types
- Overload resolution
- Extension search
- Protocol bound traversal
- Substitution application on return types
- Where clause checking at call sites
- The entire `get_type_container()` machinery

#### New expression kinds

Instead of the binder producing fully-resolved `Expression::Call { method: resolved_id, ... }`, it produces:

```rust
ExprKind::UnresolvedCall {
    receiver: Expression,
    method_name: String,
    arguments: Vec<CallArgument>,
    span: Span,
}

ExprKind::UnresolvedMemberAccess {
    receiver: Expression,
    member: String,
    span: Span,
}
```

The inference phase resolves each of these, producing the final `Call`, `FieldAccess`, `MethodRef`, etc. as it solves constraints. The `TypeOracle::resolve_member()` becomes the single source of truth.

#### Benefits

1. **Eliminates ~6000 lines** of member/call resolution from the binder — not deduplicated, *deleted*
2. **Fixes a whole class of bugs** where BIND picks the wrong overload because it doesn't have full type context
3. **Better inference** — the solver sees the whole function at once, so it can resolve `x.foo()` even when the type of `x` depends on something later
4. **One place for error messages** — no more "did the binder report this, or did inference?" confusion
5. **The TypeOracle is already the right abstraction** — it just becomes the *only* resolution path

#### Risks and mitigations

**Error quality** — The binder currently gives specific errors with full syntactic context. The solver works with constraints, so errors can be less direct. Mitigation: attach source spans and descriptive context to every constraint so error messages stay good. This is a known-solved problem (Swift and Rust both do this).

**Solver complexity** — The solver currently handles only the "deferred" subset. Making it handle all resolution means it needs to handle simple cases too. Mitigation: simple cases (`x.knownMethod()` where `x` has a known type) are trivial constraints that resolve in one step — they don't add algorithmic complexity.

**Performance** — Running `x + 1` through a constraint solver instead of eagerly resolving is more work per call. But total work may decrease because you stop doing the work twice. Profile after migration to verify.

#### Migration steps

Each step lands independently with tests passing:

**Step 0: Clean up TypeOracle internals (~1 day)** — DONE
- Extract the repeated `apply_substitutions` → `substitute_self` → `resolve_associated_types` pattern into a helper
- Consolidate `get_type_parameter_bounds()` (type_oracle.rs), `get_type_parameter_bounds_by_id()` (calls.rs), and `get_associated_type_bounds_from_context()` (calls.rs) into queries on SemanticModel
- This directly improves the code path that's about to become the only path

**Step 1: Migrate method calls to inference (~2-3 days)** — DONE
- Change the binder to emit `DeferredMethodCall` for method calls instead of resolving
- Update the solver to handle `DeferredMethodCall` via TypeOracle
- Extracted `builtin_method_call` helper on `BodyResolutionContext` to consolidate 6 builtin protocol call sites

**Step 2: Migrate member access to inference (~2 days)** — DONE
- `x.field` and `x.method` become `DeferredMemberAccess` until inference
- The solver resolves these via `TypeOracle::resolve_member()` + `classify_member()`

**Step 3: Migrate static calls and initializers (~2 days)** — DONE
- `Type.staticMethod()` and `Type()` initializer calls become `DeferredStaticCall`/`DeferredInitCall`
- The solver handles these via TypeOracle

**Step 4: Delete dead binder code (~1 day)** — BLOCKED
- Remove `members.rs`, most of `calls.rs`, `get_type_container()` and related functions
- The body_resolver shrinks from ~17,500 lines to ~5,000
- Blocked on 5 remaining eager resolution categories in the binder

#### Files affected

| File | Action |
|------|--------|
| `body_resolver/members.rs` (3146 lines) | Delete entirely |
| `body_resolver/calls.rs` (3156 lines) | Gut to ~200 lines (simple call expr construction) |
| `body_resolver/paths.rs` (1620 lines) | Simplify — keep variable/import resolution, remove method resolution |
| `body_resolver/expressions.rs` (3675 lines) | Simplify — emit unresolved exprs instead of resolving |
| `type_oracle.rs` (4181 lines) | Clean up, becomes the single resolution path |
| `kestrel-semantic-type-inference/src/solver.rs` | Add handling for `UnresolvedCall`, `UnresolvedMemberAccess` |
| `kestrel-semantic-type-inference/src/constraint.rs` | Add new constraint variants if needed |
| `kestrel-semantic-type-inference/src/apply.rs` | Update solution application for new expr kinds |
| `kestrel-semantic-tree/src/expr.rs` | Add `UnresolvedCall`, `UnresolvedMemberAccess` variants |

---

## 2. Unified Type Transformation Pipeline

### Problem

The codebase has 10 separate mechanisms for type substitution/replacement scattered across 5 crates:

| Mechanism | Location | What it replaces |
|---|---|---|
| `Substitutions::apply()` | `kestrel-semantic-tree/ty/substitutions.rs` | TypeParameter → concrete |
| `Ty::substitute_self()` | `kestrel-semantic-tree/ty/mod.rs` | Self → concrete |
| `Ty::expand_aliases()` | `kestrel-semantic-tree/ty/mod.rs` | TypeAlias → underlying |
| `resolve_associated_type()` | `kestrel-semantic-model/type_oracle.rs` | T.Item → concrete |
| `resolve_type()` | `kestrel-semantic-type-inference/apply.rs` | Infer → solved |
| `apply_protocol_defaults()` | `kestrel-semantic-model/type_oracle.rs` | default params → Self |
| `Substitution::apply_ty()` | `kestrel-codegen-cranelift/monomorphize/substitute.rs` | MirTypeParam → concrete |
| inline substitution chains | `body_resolver/calls.rs`, `body_resolver/members.rs` | ad-hoc combos of the above |

The three-step pattern `apply_substitutions` → `substitute_self` → `resolve_associated_types` is repeated ~10 times but never abstracted.

### Solution

Create a `TypeTransformer` that composes all replacement operations into a single entry point.

```rust
/// A single entry point for all type normalization.
struct TypeTransformer<'a> {
    substitutions: &'a Substitutions,
    self_type: Option<&'a Ty>,
    oracle: Option<&'a dyn TypeOracle>,
}

impl TypeTransformer {
    /// Apply ALL transformations in the correct order:
    /// 1. Expand type aliases
    /// 2. Substitute generic parameters
    /// 3. Substitute Self
    /// 4. Resolve associated types
    /// 5. Normalize (recurse until fixpoint)
    fn transform(&self, ty: &Ty) -> Ty { ... }

    /// Transform a whole callable signature at once
    fn transform_callable(&self, callable: &CallableBehavior) -> CallableBehavior { ... }
}
```

Every call site that currently does the three-step dance becomes:

```rust
let transformer = TypeTransformer::new(&subs, Some(&receiver_ty), Some(oracle));
let return_ty = transformer.transform(callable.return_type());
```

This also builds on the `TypeTransformer` trait proposed in `refactor.md` — the `map_children` method on `Ty` eliminates the duplicated structural recursion across `substitute_self`, `apply_substitutions`, `expand_aliases`, etc.

### Migration

This refactoring is independent and can happen before, during, or after the inference migration:

1. Add `Ty::map_children()` method for structural recursion
2. Implement `TypeTransformer` struct in `kestrel-semantic-tree`
3. Replace inline substitution chains in `type_oracle.rs` with `TypeTransformer` calls
4. Replace inline chains in `body_resolver/` (these disappear anyway if inference migration happens first)
5. Verify MIR-level `Substitution::apply_ty()` can use the same pattern (different type representation, but same structure)

---

## 3. Parser Rewrite: Hand-Written Recursive Descent

### Problem

The parser uses Chumsky (a combinator library) but fights it at every turn:

**`expr/mod.rs` is 3200 lines** — a single recursive Chumsky expression with deeply nested `.then().or().map()` chains. Array/dictionary disambiguation alone is 180 lines (697–875) of nested alternatives. Parenthesis disambiguation is another 70 lines.

**Three-layer boilerplate** — every grammar rule requires three functions:
1. `foo_parser_internal()` — Chumsky combinator returning raw data
2. `emit_foo()` — converts data to syntax events
3. `parse_foo()` — coordinates the above

Plus intermediate data types (`FooData`, `ExprVariant`, `BracketContent`, `BracketContentAfterFirst`, `ParenContent`, etc.) that exist solely to shuttle data between layers.

**Duplicated parsing logic:**
- `skip_trivia()` defined in 5 separate modules
- Path segments parsed 3 different ways (expr, ty, common)
- Literal parsing duplicated between expr and pattern modules
- Postfix operation parsing duplicated between expr and condition parsers

**`declaration_item/mod.rs` uses expensive backtracking** — `try_parse()` clones tokens and creates temporary EventSinks to try each declaration parser sequentially, when a single-token lookahead would suffice for most cases.

**Compile time** — Chumsky's generic type system creates enormous monomorphized types. The 15+ `boxed()` calls in `expr/mod.rs` are hints that compile time is already a problem.

### Solution

Replace Chumsky with a hand-written recursive descent parser that emits events directly. Keep the event-driven architecture (it's good for LSP), just eliminate the combinator layer.

#### What changes

| Before (Chumsky) | After (Hand-Written) |
|---|---|
| Three functions per grammar rule | One function per grammar rule |
| Intermediate data types (`FooData`) | Direct event emission |
| `.then().or().map()` chains | `if/match` on current token |
| `boxed()` to manage type complexity | No generic type issues |
| `try_parse()` backtracking for declarations | Single-token lookahead |
| Separate skip_trivia per module | One `skip_trivia()` method on parser |

#### Example: struct declaration

Before (three functions + data type):

```rust
struct StructDeclarationData {
    visibility: Option<(Token, Span)>,
    keyword_span: Span,
    name_span: Span,
    // ... more fields
}

fn struct_declaration_parser_internal() -> impl Parser<...> {
    visibility_parser_internal()
        .then(just(Token::Struct).map_with_span(|_, s| s))
        .then(identifier_parser_internal())
        // ... more combinators
        .map(|((vis, kw), name)| StructDeclarationData { ... })
}

fn emit_struct_declaration(sink: &mut EventSink, data: &StructDeclarationData) {
    sink.start_node(SyntaxKind::StructDeclaration);
    emit_visibility(sink, &data.visibility);
    sink.add_token(SyntaxKind::Struct, data.keyword_span.clone());
    // ...
    sink.finish_node();
}

pub fn parse_struct_declaration<I>(source: &str, tokens: I, sink: &mut EventSink) { ... }
```

After (one function):

```rust
fn parse_struct_declaration(&mut self) {
    self.start_node(SyntaxKind::StructDeclaration);
    self.parse_visibility();
    self.expect(Token::Struct);
    self.parse_name();
    self.parse_type_params_opt();
    self.parse_conformances_opt();
    self.parse_where_clause_opt();
    self.parse_struct_body();
    self.finish_node();
}
```

#### Expression parsing

Keep Pratt parsing for binary expressions (it's the right algorithm), just implement it directly:

```rust
fn parse_expression(&mut self) {
    self.parse_expression_bp(0) // bp = binding power
}

fn parse_expression_bp(&mut self, min_bp: u8) {
    self.start_node(SyntaxKind::Expression);
    self.parse_prefix_expression();

    while let Some((left_bp, right_bp)) = self.infix_binding_power() {
        if left_bp < min_bp { break; }
        let op = self.bump(); // consume operator
        self.parse_expression_bp(right_bp);
    }
    self.finish_node();
}
```

#### Error recovery

Hand-written parsers make error recovery trivial:

```rust
fn parse_struct_body(&mut self) {
    self.start_node(SyntaxKind::StructBody);
    if !self.expect(Token::LBrace) {
        // Recovery: skip to next brace or declaration keyword
        self.recover_to(&[Token::LBrace, Token::RBrace, Token::Struct, Token::Fn]);
    }
    while !self.at(Token::RBrace) && !self.at_eof() {
        self.parse_member_declaration();
    }
    self.expect(Token::RBrace);
    self.finish_node();
}
```

#### Benefits

1. **Eliminate ~5000 lines of boilerplate** (intermediate data types, emit functions, three-layer coordination)
2. **Dramatically improve parser compile time** (no more Chumsky generics)
3. **Adding new syntax becomes 3x faster** — one function instead of three + data type
4. **Better error recovery** — hand-written recovery strategies per grammar rule
5. **Debuggable** — step through parsing in a debugger
6. **LSP-ready** — resilient parsing that always produces a tree, even with errors

#### Parser struct design

```rust
pub struct Parser<'src> {
    tokens: Vec<(Token, Span)>,
    pos: usize,
    source: &'src str,
    events: Vec<Event>,
    file_id: usize,
}

impl<'src> Parser<'src> {
    // Core operations
    fn current(&self) -> Token { ... }
    fn bump(&mut self) -> Span { ... }           // consume current token
    fn expect(&mut self, token: Token) -> bool { ... }  // consume or error
    fn at(&self, token: Token) -> bool { ... }

    // Tree building (same events as before — compatible with Rowan)
    fn start_node(&mut self, kind: SyntaxKind) { ... }
    fn finish_node(&mut self) { ... }

    // Error recovery
    fn error(&mut self, msg: &str) { ... }
    fn recover_to(&mut self, tokens: &[Token]) { ... }
    fn skip_trivia(&mut self) { ... }
}
```

This produces the same `Event` stream that feeds into `TreeBuilder` → Rowan `SyntaxNode`. Everything downstream of the parser (syntax tree, builders, binders) is unchanged.

#### Migration

The parser rewrite is independent of the inference migration and can happen in parallel:

1. Create `Parser` struct with core operations and event emission
2. Port declaration parsing (struct, enum, protocol, function, etc.) — one at a time
3. Port statement parsing
4. Port expression parsing (biggest piece — Pratt parser + postfix operations)
5. Port pattern and type parsing
6. Delete Chumsky dependency and all intermediate data types
7. Verify all existing parser tests pass (the syntax tree output should be identical)

---

## Implementation Order

```
┌─────────────────────┐  ┌─────────────┐  ┌──────────────────┐  ┌──────────────────┐
│ 0. Clean up Oracle  │  │ TypeTransfmr│  │ Parser rewrite   │  │ Mangling rewrite │
│    DONE ✓           │  │ (can happen │  │ (independent,    │  │ DONE ✓           │
├─────────────────────┤  │  anytime)   │  │  can run in      │  └──────────────────┘
│ 1. Migrate method   │  │  ~2 days    │  │  parallel)       │
│    calls  DONE ✓    │  └─────────────┘  │  ~1-2 weeks      │
├─────────────────────┤                   └──────────────────┘
│ 2. Migrate member   │
│    access DONE ✓    │
├─────────────────────┤
│ 3. Migrate static   │
│    calls  DONE ✓    │
├─────────────────────┤
│ 4. Delete dead      │
│    binder code      │
│    BLOCKED          │
└─────────────────────┘
```

Steps 0–3 of the inference migration and the mangling rewrite are complete. Step 4 (deleting dead binder code) is blocked on 5 remaining eager resolution categories. The TypeTransformer and parser rewrite are not yet started.

### Estimated total impact

| Area | Before | After | Change |
|------|--------|-------|--------|
| `body_resolver/` | 17,475 lines | ~5,000 lines | -12,475 |
| `type_oracle.rs` | 4,181 lines | ~3,000 lines | -1,181 |
| `kestrel-parser/` | 16,925 lines | ~10,000 lines | -6,925 |
| Type inference solver | ~2,500 lines | ~3,500 lines | +1,000 |
| Mangling + name.rs | ~640 lines | ~800 lines | +160 (adds demangler) |
| **Net** | **~41,700 lines** | **~22,300 lines** | **~-19,400** |

The solver grows by ~1000 lines to handle all resolution, mangling grows slightly to add the demangler, but ~20,000 lines of duplicated/boilerplate code is eliminated.

---

## 4. Symbol Mangling Rewrite — DONE

### Problem

The current mangling scheme has several design issues that make it fragile, ambiguous, and hard to work with.

#### Ambiguous encoding

`P` means both "Pointer" (in type position: `Pi` = `*I64`) and "Parameters" (in function signature: `P2ib` = 2 params). The primitive encoding `i` means I64, but `i8`, `i16`, `i32` are multi-character — a demangler reading `i32` can't tell if it's `i` (I64) followed by `32` (a 32-byte identifier length prefix) or `i32` (the I32 type). `S` means SelfType in types, `S_` means "with self type" in function signatures.

#### Two-level naming with ad-hoc hacks

`name.rs` builds `QualifiedName` from semantic symbols using `$`-separated labels (`init$intLiteral`, `foo$x$y`). Then `mangle.rs` mangles those qualified names into linker symbols. The label encoding is ad-hoc — there's special-case detection for `from` labels to include types for disambiguation. Extensions that can't resolve their target type get `"(extension)"` in their path (parentheses aren't even valid in the scheme).

#### No demangling

There's no demangler, so `nm`, debuggers, and crash reports show raw `_K3std4core3add` strings. The scheme wasn't designed to be reversibly parseable.

#### Statics aren't mangled

Static variables bypass the mangler entirely (line 117 in context.rs: `format!("{}", name)`), risking collisions with C symbols or other Kestrel modules.

#### Duplicate type-to-string logic

`mangle_type_name()` in `name.rs` is a separate, simpler type-to-string function used only for init overload disambiguation — duplicating what the mangler does.

### Solution

A new mangling scheme designed from scratch to be:
1. **Unambiguous** — every mangled string has exactly one parse (LL(1) grammar)
2. **Demangle-able** — can reconstruct human-readable names
3. **Self-contained** — no pre-mangling step, no `$`-encoded labels in qualified names
4. **Complete** — handles all symbols (functions, statics, types, witnesses)

### Mangling Scheme v0

#### Symbol format

```
<symbol> ::= '_K0' <path> <sig>? <inst>? <self>?
```

- `_K` — Kestrel prefix
- `0` — scheme version (allows future revisions without breaking old binaries)
- `<path>` — qualified name (module.Type.method)
- `<sig>` — optional function signature (for overload disambiguation)
- `<inst>` — optional generic instantiation
- `<self>` — optional Self type (for protocol extension methods)

#### Paths

```
<path>    ::= <ident>                    -- single segment
            | 'N' <ident>+ 'E'          -- nested path (2+ segments)

<ident>   ::= <decimal-length> <utf8-bytes>
```

Paths are always length-prefixed identifiers. Multi-segment paths are wrapped in `N...E`. Single-segment paths are bare (no wrapper, saves 2 bytes for the common case of local names in tests/debugging).

Examples:
```
3add                          -- "add"
N4Main5Point3addE             -- "Main.Point.add"
N3std4core5Int645toStringE    -- "std.core.Int64.toString"
```

#### Function signatures (overload disambiguation)

```
<sig>   ::= 'Z' <param>* 'E'

<param> ::= <type>                   -- unlabeled parameter
           | 'L' <ident> <type>      -- labeled parameter
```

`Z` opens a signature block, `E` closes it. Each parameter is either a bare type (unlabeled) or `L` + label name + type.

Examples:
```
Z E                           -- no params: fn foo()
Z i8 E                        -- one unlabeled I64 param: fn foo(_ x: Int)
Z L5value i8 E                -- one labeled param: fn foo(value x: Int)
Z L3int i8 E                  -- init(int: Int64) — for literal protocols
Z L4from i8 E                 -- init(from: Int64)
Z L4from i4 E                 -- init(from: Int32) — different overload
```

This eliminates the `$`-encoding hack in qualified names. The label is part of the mangled signature, not shoved into the path segment.

#### Generic instantiation

```
<inst> ::= 'I' <type>+ 'E'
```

Examples:
```
I i8 E                        -- [Int64]
I i8 b E                      -- [Int64, Bool]
I N3std5ArrayE I i8 E E       -- [Array[Int64]] (nested generics)
```

#### Self type

```
<self> ::= 'W' <type>
```

Used for protocol extension methods to encode which concrete type `Self` was bound to.

Example:
```
W N4Main3BoxE I i8 E          -- Self = Box[Int64]
```

#### Type encoding

All types are LL(1) — the first character (or first two for `i`/`f`) uniquely determines the production.

##### Primitives

```
<prim> ::= 'b'              -- Bool
          | 's'              -- Str
          | 'v'              -- Unit (void)
          | 'n'              -- Never
          | 'i' [1248]       -- integer (i1=I8, i2=I16, i4=I32, i8=I64)
          | 'f' [248]        -- float (f2=F16, f4=F32, f8=F64)
```

`i` and `f` are ALWAYS followed by a width digit. No standalone `i` or `f`.

##### Compound types

```
<compound> ::= 'P' <type>                       -- Pointer
             | 'R' <type>                        -- Ref (borrow)
             | 'M' <type>                        -- MutRef
             | 'T' <type>* 'E'                   -- Tuple (0+ elements)
             | 'A' <type>                        -- Array
             | 'F' <decimal-count> '_' <type>* <type> 'E'  -- thin Function
             | 'C' <decimal-count> '_' <type>* <type> 'E'  -- Closure (thick)
             | 'S'                               -- Self type
             | 'X'                               -- Error type
```

Function types use a decimal count followed by `_` separator, then that many parameter types, then the return type, then `E`. This avoids backtracking.

```
F0_vE                         -- fn() -> Unit  (0 params)
F1_i8i8E                      -- fn(I64) -> I64  (1 param)
F2_i8bi8E                     -- fn(I64, Bool) -> I64  (2 params)
C1_i8i8E                      -- closure (I64) -> I64
```

##### Named types

```
<named> ::= <ident> <generic-args>?             -- single-segment name
           | 'N' <ident>+ 'E' <generic-args>?   -- multi-segment name

<generic-args> ::= 'I' <type>+ 'E'
```

Named types reuse the path encoding. In type position, a digit always starts a named type (since no other type production starts with a digit).

```
5Int64                         -- Int64 (single segment, no generics)
3BoxIi8E                       -- Box[I64]
N3std5ArrayEIi8E               -- std.Array[I64]
3BoxI3BoxIi8EE                 -- Box[Box[I64]]
```

##### Associated type projection

```
<assoc> ::= 'Q' <type> <ident>
```

`Q` for "qualified projection". The base type, then the associated type name.

```
Q 1T 4Item                    -- T.Item
Q N3std8IteratorEIi8E 4Item   -- std.Iterator[I64].Item
```

#### LL(1) parse table

Every production is determinable from the first character in type position:

| First char | Production |
|-----------|-----------|
| `b` | Bool |
| `s` | Str |
| `v` | Unit |
| `n` | Never |
| `i` | Integer (read width digit) |
| `f` | Float (read width digit) |
| `P` | Pointer |
| `R` | Ref |
| `M` | MutRef |
| `T` | Tuple (read types until `E`) |
| `A` | Array |
| `F` | Thin function (read count, `_`, params, ret, `E`) |
| `C` | Closure (same as `F`) |
| `S` | Self type |
| `X` | Error type |
| `Q` | Associated type projection |
| `N` | Multi-segment named type (read idents until `E`, then optional `I...E`) |
| `0`-`9` | Single-segment named type (read ident, then optional `I...E`) |

No ambiguity. No backtracking.

#### Full examples

```
Input: Main.identity[Int64]  (no params)
Mangled: _K0 N4Main8identityE Ii8E
         ^^^  ^^^^^^^^^^^^^^^  ^^^^
         ver  path             instantiation

Input: Main.Point.init(x: Int64, y: Int64)
Mangled: _K0 N4Main5Point4initE ZL1xi8L1yi8E
         ^^^  ^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^
         ver  path               sig (labeled x:I64, y:I64)

Input: Main.Box[Int64].wrap(_ value: Int64)  (static method)
Mangled: _K0 N4Main3Box4wrapE Zi8E Ii8E
         ^^^  ^^^^^^^^^^^^^^^  ^^^^  ^^^^
         ver  path             sig   instantiation

Input: std.Array[Int64].append(mutating self, _ element: Int64)
Mangled: _K0 N3std5Array6appendE Zi8E Ii8E
         (self param excluded from signature — it's implicit)

Input: Protocol extension method: Equatable.notEquals on Int64
Mangled: _K0 N3std9Equatable9notEqualsE ZRi8E W5Int64
         ^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^  ^^^^^^
         ver  path                       sig    self=Int64

Input: static variable Main.counter
Mangled: _K0 N4Main7counterE
         (statics use same path mangling, no sig/inst needed)
```

#### Demangling

The format is designed for straightforward recursive-descent demangling:

```rust
pub fn demangle(mangled: &str) -> Option<String> {
    let input = mangled.strip_prefix("_K0")?;
    let mut parser = DemangleParser::new(input);
    let path = parser.parse_path()?;      // "Main.Point.init"
    let sig = parser.parse_sig()?;        // "(x: Int64, y: Int64)"
    let inst = parser.parse_inst()?;      // "[Int64]"
    let self_ty = parser.parse_self()?;   // " for Int64"
    Some(format!("{}{}{}{}", path, inst, sig, self_ty))
}
```

Output examples:
```
_K0N4Main8identityEIi8E          → Main.identity[Int64]
_K0N4Main5Point4initEZL1xi8L1yi8E → Main.Point.init(x: Int64, y: Int64)
_K0N3std5Array6appendEZi8EIi8E   → std.Array[Int64].append(Int64)
```

### Migration

#### What changes

| Component | Change |
|-----------|--------|
| `kestrel-codegen/src/mangle.rs` | Rewrite with new scheme |
| `kestrel-codegen/src/lib.rs` | Add `demangle.rs`, update exports |
| `kestrel-execution-graph-lowering/src/name.rs` | Remove `$`-encoded labels from qualified names. `init$intLiteral` → just `init`. Remove `mangle_type_name()`. |
| `kestrel-codegen-cranelift/src/context.rs` | Pass `FunctionDef` param info to mangler. Mangle statics properly. |
| `kestrel-codegen-cranelift/src/rvalue.rs` | Update mangling calls |

#### Steps

1. **Write the new `Mangler`** implementing the v0 scheme (~1 day)
2. **Write the `Demangler`** with tests round-tripping every mangled symbol (~1 day)
3. **Clean up `name.rs`** — remove `$` hacks, `mangle_type_name()`, `init_name_suffix_from_callable()`. Qualified names become clean path segments. (~0.5 day)
4. **Update codegen callsites** — pass param info to new mangler, mangle statics (~0.5 day)
5. **Add a `kestrel demangle` CLI subcommand** for piping `nm` output through (~0.5 day)
6. **Run full test suite** and verify all linking still works (~0.5 day)

Total: ~4 days

This is fully independent of the other three refactors and can happen at any time.
