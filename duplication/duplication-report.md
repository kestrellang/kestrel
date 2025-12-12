# Kestrel Codebase Duplication Report

Consolidated analysis of code duplication across the Kestrel compiler's semantic analysis layers.

---

## Executive Summary

| Area | Estimated Duplicated Lines | Priority |
|------|---------------------------|----------|
| Binder Module (`kestrel-semantic-tree-binder`) | ~1,300+ lines | High |
| Model Queries (`kestrel-semantic-model`) | ~400+ lines | Medium |
| Cross-Crate Duplication (Binder ↔ Model) | ~300+ lines | Medium |
| **Total** | **~2,000+ lines** | |

---

## Part 1: Binder Duplication

### 1.1 Generic Resolution (`resolve_generics`) — ~140 lines

Nearly identical implementation in 4 files:

| File | Location |
|------|----------|
| `function.rs` | Lines 87-119 |
| `struct.rs` | Lines 61-92 |
| `protocol.rs` | Lines 72-100 |
| `type_alias.rs` | Lines 266-297 |

**Pattern:**
1. Query symbol by `context_id`
2. Filter children by `KestrelSymbolKind::TypeParameter`
3. Downcast to `Arc<TypeParameterSymbol>`
4. Call `resolve_where_clause`
5. Return `GenericsBehavior::new(type_parameters, where_clause)`

**Recommendation:** Single shared function in utils module.

---

### 1.2 Where Clause Resolution (`resolve_where_clause`) — ~120 lines

Same 4 files as above:

| File | Location |
|------|----------|
| `function.rs` | Lines 122+ |
| `struct.rs` | Lines 95+ |
| `protocol.rs` | Lines 103+ |
| `type_alias.rs` | Lines 300+ |

**Pattern:**
1. Find `WhereClause` child node
2. Iterate children for `TypeBound` nodes
3. Call `resolve_type_bound` for each
4. Return `WhereClause::with_constraints(constraints)`

**Variation:** `function.rs` also handles `TypeEquality` constraints.

**Recommendation:** Single function with optional `TypeEquality` support flag.

---

### 1.3 Type Bound Resolution (`resolve_type_bound`) — ~450 lines ⚠️ WORST OFFENDER

Duplicated across 5 files with varying complexity:

| File | Lines | Features |
|------|-------|----------|
| `function.rs` | 171-348 | Full: AssociatedTypeTarget, TypeEquality, generic protocol detection |
| `struct.rs` | 123-225 | Basic: Name extraction, path resolution |
| `protocol.rs` | 137-225 | Extended: inherited protocol lookup |
| `extension.rs` | 442-544 | Basic: uses `referenced_params` instead of `type_params` |
| `type_alias.rs` | 328-430 | Basic: same as struct.rs |

**Core pattern in all:**
```rust
// 1. Find Name node, extract identifier
let name_node = find_child(syntax, SyntaxKind::Name)?;
let name_token = name_node.children_with_tokens()
    .filter_map(|e| e.into_token())
    .find(|t| t.kind() == SyntaxKind::Identifier)?;
let param_name = name_token.text().to_string();

// 2. Look up type parameter
let param_id = type_params.iter()
    .find(|p| p.metadata().name().value == param_name)
    .map(|p| p.metadata().id());

// 3. Resolve Path children to protocol types
let bounds: Vec<Ty> = syntax.children()
    .filter(|c| c.kind() == SyntaxKind::Path)
    .map(|path_node| { /* resolve path */ })
    .collect();

// 4. Return constraint
match param_id {
    Some(id) => Some(Constraint::type_bound(id, param_name, param_span, bounds)),
    None => Some(Constraint::unresolved_type_bound(param_name, param_span, bounds)),
}
```

**Recommendation:** Create configurable `resolve_type_bound` with options for:
- AssociatedTypeTarget handling
- Generic protocol detection
- Custom type parameter source

---

### 1.4 Protocol Path Resolution Error Handling — ~180 lines

Identical pattern in 4 files (`struct.rs`, `protocol.rs`, `extension.rs`, `type_alias.rs`):

```rust
match ctx.model.query(ResolveTypePath { path: segments, context: context_id }) {
    TypePathResolution::Resolved(resolved_ty) => match resolved_ty.kind() {
        TyKind::Protocol { .. } => resolved_ty,
        TyKind::Struct { symbol, .. } => {
            ctx.diagnostics.throw(NotAProtocolError { ... });
            Ty::error(span)
        }
        TyKind::TypeAlias { symbol, .. } => {
            ctx.diagnostics.throw(NotAProtocolError { ... });
            Ty::error(span)
        }
        _ => { ctx.diagnostics.throw(NotAProtocolError { ... }); Ty::error(span) }
    },
    TypePathResolution::NotFound { .. } => {
        ctx.diagnostics.throw(UnresolvedTypeError { ... });
        Ty::error(span)
    }
    TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
        ctx.diagnostics.throw(NotAProtocolError { ... });
        Ty::error(span)
    }
}
```

**Recommendation:** Extract to `resolve_protocol_bound_path(segments, context_id, span, ctx) -> Ty`.

---

### 1.5 Parameter Resolution — ~200 lines

Duplicated between `function.rs` and `initializer.rs`:

| Function | function.rs | initializer.rs |
|----------|-------------|----------------|
| `resolve_parameters_from_syntax` | 653-674 | 199-221 |
| `resolve_single_parameter` | 676-730 | 223-288 |

**Key difference:** `initializer.rs` treats single-name parameters as having implicit labels.

**Recommendation:** Shared function with `implicit_labels: bool` parameter.

---

### 1.6 Body Resolution Setup — ~240 lines

Similar pattern in `function.rs:529-650` and `initializer.rs:77-181`:

1. Downcast to specific symbol type
2. Query symbol from model
3. Create temporary `FunctionSymbol` for `LocalScope`
4. Create `LocalScope`
5. Optionally bind `self` parameter
6. Bind regular parameters
7. Create `BodyResolutionContext`
8. Call `resolve_body`
9. Attach `ExecutableBehavior`

**Additional location:** `body_resolver/context.rs:204` (`resolve_and_attach_body`)

**Recommendation:** Extract to shared `setup_and_resolve_body(symbol, body_node, params, self_type, context)`.

---

### 1.7 Temporary FunctionSymbol Creation — ~60 lines

Multiple call sites create a dummy `FunctionSymbol` because `LocalScope::new` requires `Arc<FunctionSymbol>`:

| Location | File:Line |
|----------|-----------|
| Function binder | `function.rs:561` |
| Initializer binder | `initializer.rs:112` |
| Body resolver | `context.rs:264` |

**Recommendation:** Refactor `LocalScope::new` API or create shared helper.

---

### 1.8 Self Type Helper (`get_self_type`) — ~40 lines

Duplicated between:
- `function.rs:754-775`
- `initializer.rs:183-197`

```rust
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    let parent = symbol.metadata().parent()?;
    let parent_span = parent.metadata().span().clone();
    match parent.metadata().kind() {
        KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol => Some(Ty::self_type(parent_span)),
        KestrelSymbolKind::Extension => Some(Ty::self_type(parent_span)),
        _ => None,
    }
}
```

**Recommendation:** Single shared function.

---

### 1.9 Type Display Utilities — ~40 lines

Two similar functions:
- `type_alias.rs:458-478` — `get_type_display_name(ty: &Ty) -> String`
- `extension.rs:340-354` — `format_type_kind(kind: &TyKind) -> String`

**Recommendation:** Single `format_type_for_display(ty: &Ty) -> String` in shared utils.

---

### 1.10 Path Segment Extraction — Conceptual Duplication

Multiple custom implementations:
- `function.rs:507` — `extract_path_from_node` (for associated-type targets)
- `body_resolver/paths.rs:144` — `extract_path_segments_with_spans`
- Various binders import `kestrel_syntax_tree::utils::extract_path_segments`

**Recommendation:** Consolidate path extraction logic.

---

## Part 2: Model Query Duplication

### 2.1 Child Filtering & Extraction — ~100 lines

Three queries perform identical operations:

| Query | File |
|-------|------|
| `StructMethods` | `src/queries/struct_methods.rs` |
| `ExtensionMethods` | `src/queries/extension_methods.rs` |
| `FunctionsInSymbol` | `src/queries/functions_in_symbol.rs` |

All iterate children, filter for `Function` kind, and map to `(String, Span)`.

**Recommendation:** Generic `MethodsInSymbol` query or shared helper.

---

### 2.2 Protocol Inheritance Traversal — ~150 lines

Three queries implement their own protocol graph traversal:

| Query | File | Algorithm |
|-------|------|-----------|
| `ProtocolRequiredMethods` | `protocol_required_methods.rs` | Recursive DFS |
| `ProtocolMethodsWithDefiner` | `protocol_methods_with_definer.rs` | Iterative BFS (Queue) |
| `InheritedProtocolMember` | `inherited_protocol_member.rs` | Recursive DFS (early exit) |

**Recommendation:** Create shared `ProtocolAncestorIterator` or `ProtocolWalker` helper.

---

### 2.3 Path Resolution Boilerplate — ~100 lines

`ResolveValuePath` and `ResolveTypePath` share significant startup logic:

1. Check for empty path
2. Resolve first segment using `ResolveName`
3. Handle `SymbolResolution` enums (`Found`, `Ambiguous`, `NotFound`)
4. Resolve intermediate segments using `VisibleChildrenByName`

**Recommendation:** Extract `PathResolver` utility for shared segment resolution.

---

### 2.4 Ancestor Traversal — ~50 lines

Two queries duplicate ancestor walking:
- `AncestorOfKind` (`ancestor_of_kind.rs`)
- `IsInsideAny` (`is_inside_any.rs`)

**Recommendation:** `IsInsideAny` should call `AncestorOfKind` or share a `walk_ancestors` iterator.

---

## Part 3: Cross-Crate Duplication (Binder ↔ Model)

### 3.1 Protocol Inheritance Traversal

| Crate | Location | Purpose |
|-------|----------|---------|
| Binder | `protocol_flattener.rs` | Creates `FlattenedProtocolBehavior` cache |
| Model | `protocol_methods_with_definer.rs`, etc. | Finds methods on demand |

**Issue:** Model queries ignore the `FlattenedProtocolBehavior` cache and re-traverse the inheritance graph every time.

**Recommendation:**
1. Create `ProtocolAncestorIterator` in Model
2. Binder's `protocol_flattener` uses this iterator to build cache
3. Model queries check cache first, then fall back to iterator

---

### 3.2 Associated Type Resolution (`T.Item`)

| Crate | Location | Implementation |
|-------|----------|----------------|
| Binder | `function.rs` (`resolve_type_bound`) | Manual iteration over constraints and protocol children |
| Model | `resolve_type_path.rs` (`resolve_associated_type_from_type_param_with_context`) | Same manual iteration |

**Recommendation:** Expose `resolve_associated_type_from_type_param` as a Model Query; Binder calls this instead of manual iteration.

---

### 3.3 Extension Applicability

| Crate | Location |
|-------|----------|
| Binder | `body_resolver/members.rs` (`filter_applicable_extensions`, `is_extension_applicable`) |
| Model | `ExtensionRegistry` exists but lacks filtering logic |

**Issue:** If any other compiler component needs "Which extensions apply to this type?", they must duplicate `body_resolver/members.rs` logic.

**Recommendation:** Move `filter_applicable_extensions` to Model as `ApplicableExtensionsFor { type: Ty }` query.

---

## Recommended File Structure

```
lib/kestrel-semantic-tree-binder/src/binders/
├── mod.rs
├── utils/
│   ├── mod.rs
│   ├── generics.rs       # resolve_generics, resolve_where_clause, resolve_type_bound
│   ├── parameters.rs     # resolve_parameters_from_syntax, resolve_single_parameter
│   ├── body.rs           # setup_body_resolution_context, get_self_type
│   ├── type_paths.rs     # resolve_protocol_bound_path
│   └── display.rs        # format_type_for_display
├── extension.rs
├── field.rs
├── function.rs
├── import.rs
├── initializer.rs
├── module.rs
├── protocol.rs
├── protocol_flattener.rs
├── struct.rs
├── terminal.rs
└── type_alias.rs

lib/kestrel-semantic-model/src/queries/
├── utils/
│   ├── mod.rs
│   ├── protocol_walker.rs    # ProtocolAncestorIterator
│   ├── path_resolver.rs      # Shared path resolution logic
│   └── child_filter.rs       # Generic child extraction
├── ... (existing query files)
```

---

## Implementation Priority

### High Impact (630+ lines)
1. `resolve_type_bound` consolidation
2. Protocol path resolution error handling

### Medium Impact (560+ lines)
3. `resolve_generics` + `resolve_where_clause`
4. Parameter and body resolution
5. Protocol inheritance traversal (Model queries)

### Lower Impact (200+ lines)
6. `get_self_type` and display utilities
7. Child filtering queries
8. Path resolution boilerplate

---

## Implementation Notes

- The `function.rs` version of `resolve_type_bound` is the most complete and should serve as the base implementation
- Some variations exist for good reasons (e.g., initializer implicit labels) — use configuration flags
- Consider a builder pattern for `resolve_type_bound` options if complexity grows
- Cross-crate refactoring should prioritize cache utilization to improve performance
