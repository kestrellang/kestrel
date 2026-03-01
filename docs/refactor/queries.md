# Incremental Compilation via Query Architecture

Moving the Kestrel compiler toward incremental compilation through a query-based architecture, combining HECS (Hierarchical Entity Component System) principles with Salsa-style memoized queries.

---

## Table of Contents

- [Vision](#vision)
- [Current State](#current-state)
- [Design: HECS + Salsa](#design-hecs--salsa)
- [Existing Queries](#existing-queries)
- [New Queries by Tier](#new-queries-by-tier)
- [Migration Plan](#migration-plan)

---

## Vision

Three ideas work together:

- **HECS** — data model: entities (declarations) with components (syntax-derived facts), replacing the mutable symbol tree
- **Salsa-style queries** — computation model: memoized, dependency-tracked functions that replace imperative phase walks
- **Incremental invalidation** — when a file changes, only recompute what depends on it

The end state: no explicit BUILD/BIND/VALIDATE phases. Instead, a demand-driven query graph where asking "what are the diagnostics for function F?" triggers exactly the work needed and nothing more. Results are cached across compilations, invalidated only when inputs change.

### What This Enables

1. **Fast test suite** — cached prelude/stdlib means each test only compiles its own code
2. **LSP responsiveness** — edit a function body, only re-analyze that function and its dependents
3. **Cleaner architecture** — one resolution path (queries) instead of duplicated binder + oracle logic
4. **Shorter code** — duplicated struct/enum/protocol handling collapses into component-based queries

---

## Current State

### What Exists

- **38 queries** in `kestrel-semantic-model/src/queries/` — compute eagerly, no caching, no dependency tracking
- **Monolithic pipeline** — Lex → Parse → Build → Bind → Validate, each phase runs on everything
- **`SemanticModel::query()`** — dispatches through `Query` trait, already the right shape for memoization
- **TypeOracle** — ~5000 lines of type/member resolution on `SemanticModel`, essentially queries without the trait

### What's Missing

- No memoization — same query recomputed every time it's called
- No dependency tracking — can't know what to invalidate
- Binder mutates symbols imperatively (attach behaviors) instead of computing on demand
- Heavy operations (conformance checking, member resolution, body resolution) not expressed as queries
- Duplicated logic across struct/enum (copy semantics, conformance, field validation)

---

## Design: HECS + Salsa

### Data Model (HECS)

```
Entity = a declaration (struct, function, field, enum case, etc.)
  - stable EntityId
  - kind (struct, function, ...)
  - syntax node (CST fragment)
  - parent entity

Components = syntax-derived facts (pure, local, no cross-entity lookups)
  - Name, Span, Visibility
  - GenericParams (syntax: "T, U where T: Foo")
  - CallableSignature (syntax: param names, type annotations)
  - FieldList (syntax: field names + type annotations)
  - Conformances (syntax: ": Foo, Bar")
  - Body (syntax: unresolved expression tree)

Queries = everything requiring cross-entity knowledge
  - ResolvedType(entity) → Ty
  - ResolvedBody(entity) → resolved expression tree
  - ConformsTo(ty, protocol) → bool
  - MethodsFor(entity) → Vec<EntityId>
  - Diagnostics(entity) → Vec<Diagnostic>
```

Components are extracted from syntax alone (one entity, no lookups). Queries can depend on other entities' components and other queries — that's what makes them cacheable.

### Computation Model (Salsa)

```
source_text(file)                          ← input (you set this)
  │
  ├─→ parse(file) → SyntaxTree             ← memoized
  │     │
  │     ├─→ entities(file) → Vec<EntityId>  ← memoized
  │     │
  │     ├─→ name(entity) → String           ← component extraction
  │     ├─→ generic_params(entity) → ...    ← component extraction
  │     └─→ raw_body(entity) → ExprSyntax   ← component extraction
  │
  ├─→ resolve_type_ref(type_syntax) → Ty    ← cross-entity lookup
  │
  ├─→ resolved_signature(entity) → CallableBehavior
  │     └─→ depends on: generic_params, resolve_type_ref
  │
  ├─→ resolved_body(entity) → ResolvedBody
  │     └─→ depends on: raw_body, resolved_signature (of callees),
  │         methods_for, conforms_to, type inference
  │
  ├─→ methods_for(entity) → Vec<EntityId>
  │     └─→ depends on: entities of the type + its extensions
  │
  ├─→ conforms_to(ty, protocol) → bool
  │     └─→ depends on: methods_for, resolved_signature
  │
  └─→ diagnostics(entity) → Vec<Diagnostic>
        └─→ depends on: resolved_body, conforms_to, exhaustiveness, etc.
```

When a file changes, Salsa invalidates `source_text(file)`. If the syntax tree is structurally identical (e.g., whitespace-only change), nothing downstream reruns. If a function body changed but its signature didn't, only `resolved_body` and `diagnostics` for that function rerun — callers don't need rechecking.

### How Phases Dissolve

```
Today:                          Query architecture:

BUILD (create symbols,          "parse(file)" query → syntax tree
 attach behaviors)              "entities(file)" query → Vec<EntityId>
                                component queries: name, params, fields, etc.
                                (all pure, local extraction)

BIND (resolve types,            "resolve_type(entity)" query
 resolve bodies,                "resolve_body(entity)" query
 attach more behaviors)         "callable_signature(entity)" query
                                "resolve_conformances(entity)" query
                                (call other queries, get memoized)

VALIDATE (check errors)         "diagnostics(entity)" query
                                (calls resolve_body, conformance checks, etc.)
                                (also memoized)
```

---

## Existing Queries

42 queries in `kestrel-semantic-model/src/queries/`:

| Query | Input | Output | Call Sites |
|-------|-------|--------|------------|
| `SymbolFor` | `SymbolId` | `Option<Arc<dyn Symbol>>` | 137 |
| `ExtensionsFor` | `target_id` | `Vec<Arc<ExtensionSymbol>>` | 48 |
| `ConformancesForSymbol` | `symbol_id` | `Vec<Ty>` | 33 |
| `ResolvedAliasedType` | `type_alias_id` | `Option<Ty>` | 21 |
| `IsVisibleFrom` | `target, context` | `bool` | 9 |
| `ResolveTypePath` | `path, scope, ...` | `TypePathResolution` | 7 |
| `StructFields` | `struct_id` | `Vec<StructFieldInfo>` | 5 |
| `InheritedProtocolMember` | `struct_id, member_name` | `Option<SymbolId>` | 5 |
| `ExecutableBodyFor` | `symbol_id` | `Option<CodeBlock>` | 5 |
| `ProtocolRequiredMethods` | `protocol_id` | `Vec<(CallableSignature, Arc<FunctionSymbol>)>` | 4 |
| `AncestorOfKind` | `symbol_id, kind` | `Option<SymbolId>` | 4 |
| `StructFieldTypes` | `struct_id` | `Vec<StructFieldTypeInfo>` | 3 |
| `ResolveName` | `name, scope, ...` | `SymbolResolution` | 3 |
| `ResolveModulePath` | `path` | `Result<SymbolId, ...>` | 3 |
| `LocalName` | `symbol_id` | `Option<String>` | 3 |
| `GenericsDataFor` | `symbol_id` | `Option<GenericsData>` | 2 |
| `AllConformancesFor` | `symbol_id` | `Vec<Ty>` | 5 |
| `AllMethodsFor` | `symbol_id` | `Vec<Arc<FunctionSymbol>>` | 4 |
| `AllInitializersFor` | `symbol_id` | `Vec<Arc<InitializerSymbol>>` | 1 |
| `WhereClausesInScope` | `context_id` | `Vec<WhereClause>` | 5 |
| `TypeParameterBounds` | `param_id` | `Vec<Ty>` | 6 |
| `AssociatedTypeBindingsFor` | `symbol_id` | `HashMap<String, SignatureType>` | 4 |
| `DeclaredNamesInScope` | `scope_id` | `Vec<DeclaredName>` | 2 |
| `ProtocolMethodsWithDefiner` | `struct_id` | `Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)>` | 2 |
| `ProtocolAssociatedTypesWithDefaults` | `protocol_id` | `HashMap<String, Option<SignatureType>>` | 2 |
| `HasBody` | `symbol_id` | `Option<bool>` | 2 |
| `VisibleChildrenByName` | `parent_id, name` | `Vec<Arc<dyn Symbol>>` | 2 |
| `FunctionsInSymbol` | `symbol_id` | `Vec<Arc<FunctionSymbol>>` | — |
| `ExtensionMethods` | `symbol_id` | `Vec<(String, Span)>` | — |
| `StructMethods` | `struct_id` | `Vec<(String, Span)>` | — |
| `InferenceResultFor` | `symbol_id` | `Option<Solution>` | — |
| `ScopeFor` | `symbol_id` | `Arc<Scope>` | 1 |
| `VisibleChildren` | `parent_id` | `Vec<Arc<dyn Symbol>>` | 1 |
| `ProtocolRequiredProperties` | `protocol_id` | `Vec<PropertyRequirement>` | 1 |
| `ProtocolRequiredInitializers` | `protocol_id` | `Vec<(CallableSignature, Arc<InitializerSymbol>)>` | 1 |
| `ProtocolInitializersWithDefiner` | `struct_id` | `Vec<(Arc<ProtocolSymbol>, Arc<InitializerSymbol>)>` | 1 |
| `ImportsInScope` | `scope_id` | `Vec<Arc<Import>>` | — |
| `IsInsideAny` | `symbol_id, kinds` | `bool` | 1 |
| `ResolveValuePath` | `path, scope, ...` | `ValuePathResolution` | 1 |
| `ChildByName` | `parent_id, name` | `Option<Arc<dyn Symbol>>` | — |
| `CallableParamTypesForCall` | `symbol_id, ...` | `Option<Vec<Ty>>` | 1 |

---

## New Queries by Tier

### Tier 1 — Easy Wins (compose existing queries, deduplicate code) ✅ DONE

These have been implemented inside the current architecture with no structural changes. Each one either composes existing queries or extracts duplicated logic into a single query.

#### `AllConformancesFor(symbol_id) → Vec<Ty>`

Conformances from the symbol itself plus all its extensions. Currently computed ad-hoc in **5 separate places** in the conformance analyzer with identical code:

```rust
// This pattern appears 5 times:
let mut conformances = model.query(ConformancesForSymbol { symbol_id });
for ext in model.query(ExtensionsFor { target_id: symbol_id }) {
    conformances.extend(model.query(ConformancesForSymbol { symbol_id: ext.id() }));
}
```

Files: `analyzers/conformance/mod.rs` (lines ~235, ~602, ~944, ~1067, ~1195)

#### `AllMethodsFor(symbol_id) → Vec<Arc<FunctionSymbol>>`

All methods from the symbol plus its extensions. Currently computed ad-hoc in **3 places** in the conformance analyzer and reimplemented in the extension conflict analyzer:

Files: `analyzers/conformance/mod.rs` (lines ~263, ~647, ~1093), `analyzers/extension_conflict/mod.rs`

Composes: `FunctionsInSymbol` + `ExtensionsFor` + `FunctionsInSymbol` per extension.

#### `WhereClausesInScope(symbol_id) → Vec<WhereClause>`

Walk the parent chain collecting where clauses. Currently **duplicated** between:

- `type_oracle.rs`: `collect_where_clauses_for_context` (5 call sites)
- `analyzers/type_assignability/mod.rs`: `collect_where_clauses` (separate reimplementation)

Both do the same parent-chain walk. Pure function of the symbol tree.

#### `TypeParameterBounds(param_id) → Vec<Ty>`

All protocol bounds on a type parameter, collected from where clauses + extension where clauses up the parent chain. Currently implemented in:

- `type_oracle.rs`: `get_type_parameter_bounds_with_model` (7 call sites)
- `body_resolver/utils.rs`: `get_type_parameter_bounds_by_id` (separate reimplementation)

Both walk the full parent chain AND query `ExtensionsFor` at each level.

#### `OuterTypeParameters(symbol_id) → Vec<Arc<TypeParameterSymbol>>`

Type parameters from enclosing scopes (parent chain walk). Currently computed by `collect_outer_type_parameters` in `binders/utils/generics.rs`, called twice for the same symbol (once for chaining, once for shadowing checks).

#### `AssociatedTypeBindingsFor(symbol_id) → HashMap<String, SignatureType>`

Unify `AssociatedTypeBindingsForStruct` and `AssociatedTypeBindingsForEnum` into a single query that works for any type container. The current split is struct-vs-enum but the logic is the same shape.

### Tier 2 — Hot Path Memoization (high call frequency, expensive computation)

These would benefit most from caching. They're called many times per compilation with the same inputs, and each call does non-trivial work.

#### `ConformsTo(ty, protocol_id) → bool`

Whether a type conforms to a protocol (direct + extensions + transitive). **27 external call sites** plus internal recursion. Each call walks the full conformance graph.

Implementation: the TypeOracle's `conforms_to` method, backed by `check_transitive_conformance_impl`. Cache key would be `(SymbolId, Substitutions, SymbolId)` — normalized from the input `Ty`.

Files: `type_oracle.rs` (main impl), called from solver, analyzers, body resolver

#### `ProtocolConformancesForType(ty) → Vec<Ty>`

All protocol types (with substitutions) that a type conforms to, via BFS over the protocol graph. Backs `resolve_member_via_protocol_conformance` which is called from **8 places** in member resolution.

Files: `type_oracle.rs`: `collect_protocol_conformances_for_type`

#### `ProtocolInheritanceChain(protocol_id) → Vec<(Arc<ProtocolSymbol>, Substitutions)>`

A protocol and all its inherited parent protocols (recursively). Called from **13 places** across member resolution paths via `collect_protocols_with_inherited`.

Files: `type_oracle.rs` line ~4201

#### `CopySemanticsFor(symbol_id) → CopySemantics`

Whether a type is Copyable, Cloneable, or NotCopyable. Currently **two near-identical ~150-line functions** in the struct and enum binders. The only difference: struct iterates Field children, enum iterates EnumCase children.

Files: `binders/struct.rs` (`compute_copy_semantics`), `binders/enum_binder.rs` (`compute_copy_semantics`)

Complexity note: currently uses a `CycleDetector` threaded through `BindingContext`. A query version needs its own cycle detection.

#### `ResolveAssociatedType(container_ty, assoc_name) → Option<Ty>`

Resolves `Container.AssocName` to a concrete type. **5+ scattered implementations** across the codebase. Called during every deferred member access and recursively from `deeply_resolve_associated_types`.

Files: `type_oracle.rs` (main), `body_resolver/utils.rs` (binder version), plus inline patterns

#### `NormalizeWithConstraints(ty, context_id) → Ty`

Normalizes a type using equality constraints from where clauses in scope. Called by the solver's `normalize_with_constraints` on every type pair during unification — the hottest inner loop.

Files: `type_oracle.rs`: `normalize_type_with_context`

#### `AssociatedTypeBoundsInContext(assoc_type, context_id) → Vec<Ty>`

Protocol bounds on an associated type from its declaration + where clauses in context. **5 call sites** in type_oracle.

Files: `type_oracle.rs`: `get_associated_type_bounds_with_context`

### Tier 3 — Structural Improvements (moderate value, better architecture)

#### `FlattenedProtocolFor(protocol_id) → FlattenedProtocolBehavior`

Recursively walks protocol inheritance, collecting all methods, properties, and associated types. Currently computed eagerly in `ProtocolBinder::bind_signature` and attached as a behavior.

Files: `binders/protocol_flattener.rs`

Complexity: Hard — interleaves error emission (cycle detection) with computation. Needs separation of "compute flat structure" from "emit cycle error."

#### `SelfProtocolBounds(context_id) → Vec<SymbolId>`

Protocol IDs that `Self` is bounded by in a given context. 2 call sites in `ContextualOracle::conforms_to`.

Files: `type_oracle.rs`: `self_protocol_bounds`

#### `ConcreteSelfType(context_id) → Option<Ty>`

The concrete type that `Self` resolves to inside a struct/enum/extension method.

Files: `type_oracle.rs`: `resolve_concrete_self_type_from_context`

#### `ExtensionBoundsForParam(context_id, param_id) → Option<Vec<Ty>>`

Extra protocol bounds on a type parameter from extension where clauses in context.

Files: `type_oracle.rs`: `get_extension_bounds_for_param`

#### `IsMarkerProtocol(protocol_id) → bool`

Whether a protocol has no required methods or associated types. Simple children scan.

Files: `binders/protocol.rs`: `is_marker_protocol`

#### `VisibilityLevelOf(symbol_id) → VisibilityLevel`

Visibility level of a symbol. Currently two near-identical functions in the visibility analyzer (`get_symbol_visibility_level` and `get_visibility_level_from_symbol`).

Files: `analyzers/visibility_consistency/mod.rs`

#### `ProtocolCycleCheck(protocol_id) → Option<CyclePath>`

Whether a protocol's inheritance graph has a cycle. Currently **detected twice**: once during binding (protocol flattener) and once during analysis (conformance analyzer's `check_circular_inheritance`).

Files: `binders/protocol_flattener.rs`, `analyzers/conformance/mod.rs`

### Tier 4 — Full HECS (binder operations become queries)

These represent the end state where BUILD/BIND/VALIDATE dissolve entirely. Each is a major undertaking.

#### `ResolvedSignature(symbol_id) → CallableBehavior`

Replaces signature binding in the binder — resolve parameter types, return type, where clause bounds. Currently done imperatively in `binders/` for each declaration type.

#### `ResolvedBody(symbol_id) → ResolvedBody`

Replaces body resolution in the binder — the ~8000-line body_resolver. This is the single largest piece of work. The binder currently resolves expressions, method calls, member access, closures, patterns, and control flow in two passes over the entire tree.

#### `Diagnostics(entity) → Vec<Diagnostic>`

Replaces all validation for an entity. Calls `ResolvedBody`, conformance checks, exhaustiveness, type checking, etc. The final consumer query.

---

## Migration Plan

### Phase 1: Create Tier 1 Queries ✅ DONE

Completed. Six queries created, all callers updated, ~200 lines of duplicated code removed:

1. ✅ `AllConformancesFor` — replaced 5 duplicated sites in conformance analyzer
2. ✅ `AllMethodsFor` — replaced 4 duplicated sites in conformance analyzer
3. ✅ `AllInitializersFor` — replaced 1 site in conformance analyzer (bonus query)
4. ✅ `WhereClausesInScope` — eliminated dual implementation in oracle + type_assignability
5. ✅ `TypeParameterBounds` — replaced 6 call sites in type_oracle
6. ✅ `AssociatedTypeBindingsFor` — unified struct/enum split, deleted old variants
7. `OuterTypeParameters` — deferred to a later phase

After this phase: all code paths go through queries for these operations. The conformance analyzer shrank significantly.

### Phase 2: Create Tier 2 Queries + Add Memoization

**Effort:** ~2-3 weeks. **Impact:** Measurable compilation speedup.

Add a cache layer to `SemanticModel::query()`:

```rust
impl SemanticModel {
    pub fn query<Q: Query + Hash + Eq>(&self, query: Q) -> Q::Output
    where Q::Output: Clone
    {
        if let Some(cached) = self.cache.get(&query) {
            return cached.clone();
        }
        let result = query.execute(self);
        self.cache.insert(query, result.clone());
        result
    }
}
```

Then create and memoize the Tier 2 queries. Priority order by impact:

1. `ConformsTo` — 27 call sites, most impactful single cache
2. `ProtocolInheritanceChain` — 13 call sites, feeds into member resolution
3. `ProtocolConformancesForType` — BFS over protocol graph, expensive
4. `CopySemanticsFor` — eliminates struct/enum duplication
5. `ResolveAssociatedType` — called during every deferred member access
6. `NormalizeWithConstraints` — solver hot loop
7. `AssociatedTypeBoundsInContext` — 5 call sites

After this phase: repeated queries within a single compilation are served from cache. Test suite may see modest speedup from reduced redundant work.

### Phase 3: Tier 3 Queries + TypeOracle Migration

**Effort:** ~2-3 weeks. **Impact:** Architectural cleanup, TypeOracle becomes thin wrapper.

Convert remaining TypeOracle methods into queries. The TypeOracle trait becomes a thin dispatch layer over `SemanticModel::query()` rather than a 5000-line implementation.

Key conversions:
- `resolve_member` → `ResolveMember` query
- `classify_member` → `ClassifyMember` query
- Protocol flattening → `FlattenedProtocolFor` query
- Cycle detection → `ProtocolCycleCheck` query (eliminates dual detection)

After this phase: TypeOracle is mostly mechanical dispatch. All heavy computation is in memoized queries.

### Phase 4: Introduce Salsa + Entity IDs

**Effort:** ~1-2 months. **Impact:** True incremental compilation.

Replace hand-rolled cache with Salsa's `#[salsa::tracked]` functions:
- Automatic dependency tracking (no manual invalidation)
- Equality-based short-circuiting (unchanged results don't cascade)
- File-level inputs that trigger minimal recomputation

Introduce `EntityId` as the primary way to reference declarations:
- Assigned during syntax tree construction, stable across re-parses
- Replace `Arc<dyn Symbol>` in query signatures
- Components become Salsa-tracked structs

After this phase: editing a file only recomputes affected queries. LSP can serve diagnostics incrementally. Test suite caches stdlib/prelude across tests.

### Phase 5: Dissolve Phases

**Effort:** ~2-3 months. **Impact:** Full HECS architecture.

Convert the binder's imperative passes into Tier 4 queries:
- `ResolvedSignature(entity)` replaces signature binding
- `ResolvedBody(entity)` replaces body resolution
- `Diagnostics(entity)` replaces validation

No more BUILD → BIND → VALIDATE ordering. Everything is demand-driven through the query graph. Systems match entities by component (has fields? has generic params?) rather than by entity type (is struct? is enum?).

After this phase: the compiler is fully incremental. Adding a new declaration type means defining its components and any new queries — no phase code to modify.
