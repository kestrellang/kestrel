# Semantic Analysis Refactoring Plan

This document outlines architectural improvements to the semantic analysis portion of the Kestrel compiler, focusing on:
1. Moving toward a true hECS (hierarchical Entity Component System) architecture
2. Enabling incremental compilation through a query-based system
3. Eliminating code duplication

---

## Table of Contents

- [Current State Problems](#current-state-problems)
- [Ideal Architecture](#ideal-architecture)
- [Queries to Create](#queries-to-create)
- [Deduplication](#deduplication)
- [TyKind and ExprKind Cleanup](#tykind-and-exprkind-cleanup)
- [Migration Path](#migration-path)

---

## Current State Problems

### 1. Entity-Specific Systems Instead of Component-Based

The current architecture has behaviors (components) that can attach to any symbol, but the systems (binders, analyzers) operate on specific entity types:

```rust
// Current: Entity-type switching everywhere
match symbol.metadata().kind() {
    KestrelSymbolKind::Struct => { /* struct-specific code */ }
    KestrelSymbolKind::Enum => { /* nearly identical enum code */ }
    _ => {}
}
```

This violates hECS principles where systems should operate on components, not entity types.

### 2. Eager Binding Prevents Incremental Compilation

```rust
// Current: Walk entire tree twice
fn run_binding(&mut self) -> SemanticModel {
    self.bind_signatures(&self.root.clone());  // Pass 1
    self.bind_bodies(&self.root.clone());      // Pass 2
}
```

No memoization, no dependency tracking, no lazy evaluation.

### 3. Massive Code Duplication

| Area | Duplicated Lines | Example |
|------|------------------|---------|
| Struct vs Enum binders | ~500 lines | `compute_copy_semantics` nearly identical |
| TyKind match arms | ~500 lines | Struct/Enum/Protocol/TypeAlias patterns |
| Conformance checking | ~400 lines | `check_struct_conformance` vs `check_enum_conformance` |
| Self/type substitution | ~300 lines | Multiple `substitute_self` implementations |
| Associated type resolution | ~300 lines | 5+ implementations across codebase |

---

## Ideal Architecture

### Long-Term Vision: True ECS

#### Unified Entity ID

```rust
// EntityId stable from syntax tree through execution
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct EntityId(u64);

// Assigned during syntax tree construction, never changes
```

#### World with Component Storage

```rust
pub struct World {
    // Component storage - each component type has its own map
    syntax_nodes: HashMap<EntityId, SyntaxNode>,
    parent_of: HashMap<EntityId, EntityId>,
    children_of: HashMap<EntityId, Vec<EntityId>>,

    // Query cache (behaviors become cached queries)
    query_cache: QueryCache,
}
```

#### Behaviors Become Cached Queries

```rust
// Instead of storing GenericsBehavior on symbols:
pub fn generics_for(world: &World, entity: EntityId) -> Option<GenericsData> {
    world.cached_query(GenericsQuery { entity })
}

// Instead of storing CopySemanticsBehavior:
pub fn copy_semantics_for(world: &World, entity: EntityId) -> CopySemantics {
    world.cached_query(CopySemanticsQuery { entity })
}

// Instead of storing ExecutableBehavior:
pub fn resolved_body_for(world: &World, entity: EntityId) -> Option<ResolvedBody> {
    world.cached_query(ResolvedBodyQuery { entity })
}
```

#### Systems Instead of Analyzers

```rust
pub trait System {
    /// What entities does this system care about?
    fn matches(&self, world: &World, entity: EntityId) -> bool;

    /// Run the system, producing diagnostics
    fn run(&self, world: &World, entity: EntityId, diag: &mut Diagnostics);
}

// Example: Runs on ANY entity with fields (struct or enum)
pub struct CopySemanticsValidatorSystem;

impl System for CopySemanticsValidatorSystem {
    fn matches(&self, world: &World, entity: EntityId) -> bool {
        has_field_like_children(world, entity)  // Component-based, not type-based
    }

    fn run(&self, world: &World, entity: EntityId, diag: &mut Diagnostics) {
        // ONE implementation for both structs and enums
        let copy_semantics = copy_semantics_for(world, entity);
        // validate...
    }
}
```

#### No More Build/Bind/Validate Phases

```rust
// Current: Explicit phases
fn compile() {
    let model = build(syntax_trees);      // Phase 1
    let model = bind(model);              // Phase 2
    let diags = validate(model);          // Phase 3
}

// Ideal: Demand-driven
fn compile() {
    let world = World::from_syntax(syntax_trees);  // Just store syntax

    // Systems run, triggering queries as needed
    for entity in world.all_entities() {
        for system in systems {
            if system.matches(&world, entity) {
                system.run(&world, entity, &mut diags);
            }
        }
    }
    // Queries lazily computed and cached
}
```

#### Incremental Invalidation

```rust
impl QueryCache {
    fn invalidate_on_syntax_change(&mut self, entity: EntityId) {
        // Invalidate all queries that depend on this entity's syntax
        for query_key in self.reverse_deps.get(&entity) {
            self.invalidate(query_key);
        }
    }
}

// When user edits a struct:
// 1. Syntax component updated
// 2. type_of(struct_entity) invalidated
// 3. copy_semantics_for(struct_entity) invalidated
// 4. Systems re-run only on affected entities
```

---

## Queries to Create

### Phase 1: Core Type Queries

These replace direct behavior lookups and enable caching:

```rust
/// Get generics data for any symbol (struct, enum, function, protocol, etc.)
pub struct GenericsFor { pub symbol_id: SymbolId }
// Output: Option<GenericsData>
// Replaces: symbol.type_parameters(), symbol.where_clause(), symbol.is_generic()

/// Get the type of a symbol
pub struct TypeOf { pub symbol_id: SymbolId }
// Output: Option<Ty>
// Replaces: TypedBehavior lookups

/// Get callable signature for any callable symbol
pub struct CallableFor { pub symbol_id: SymbolId }
// Output: Option<CallableData>
// Replaces: CallableBehavior lookups

/// Get copy semantics for a type container
pub struct CopySemanticsFor { pub symbol_id: SymbolId }
// Output: CopySemantics
// Replaces: CopySemanticsBehavior lookups
```

### Phase 2: Conformance Queries

```rust
/// Check if a type conforms to a protocol (cached)
pub struct ConformsTo { pub ty: Ty, pub protocol_id: SymbolId }
// Output: bool
// Replaces: 6+ implementations of conforms_to

/// Get all conformances for a type (including extensions)
pub struct AllConformancesFor { pub symbol_id: SymbolId }
// Output: Vec<Ty>
// Replaces: ConformancesForSymbol + extension lookups

/// Get all methods for a type (including extensions)
pub struct MethodsFor { pub symbol_id: SymbolId }
// Output: Vec<Arc<FunctionSymbol>>
// Replaces: manual child iteration + extension iteration
```

### Phase 3: Where Clause Queries

```rust
/// Get all where clauses in scope for a context
pub struct WhereClausesInScope { pub context_id: SymbolId }
// Output: Vec<WhereClause>
// Replaces: collect_where_clauses, walking parent chain

/// Get bounds for a type parameter
pub struct TypeParameterBounds { pub param_id: SymbolId }
// Output: Vec<Ty>
// Replaces: get_type_parameter_bounds (2+ implementations)
```

### Phase 4: Associated Type Queries

```rust
/// Resolve an associated type on a container
pub struct ResolveAssociatedType { pub container: Ty, pub assoc_name: String }
// Output: Option<Ty>
// Replaces: 5+ implementations of resolve_associated_type

/// Get associated type bindings for a type
pub struct AssociatedTypeBindingsFor { pub symbol_id: SymbolId }
// Output: HashMap<String, SignatureType>
// Replaces: AssociatedTypeBindingsForStruct + AssociatedTypeBindingsForEnum
```

### Phase 5: Unified Type Container Query

```rust
/// Get symbol and substitutions for any nominal type
pub struct NominalTypeInfo { pub ty: Ty }
// Output: Option<(SymbolId, Substitutions)>
// Replaces: get_type_container_with_subs, get_type_symbol_id patterns
```

---

## Deduplication

### 1. TyKind: Unify Nominal Types

**Current:** 4 nearly identical variants

```rust
pub enum TyKind {
    Struct { symbol: Arc<StructSymbol>, substitutions: Substitutions },
    Enum { symbol: Arc<EnumSymbol>, substitutions: Substitutions },
    Protocol { symbol: Arc<ProtocolSymbol>, substitutions: Substitutions },
    TypeAlias { symbol: Arc<TypeAliasSymbol>, substitutions: Substitutions },
    // ...
}
```

**Proposed:** Single `Nominal` variant

```rust
pub enum TyKind {
    Nominal {
        symbol_id: SymbolId,
        substitutions: Substitutions,
    },
    // ... other variants unchanged
}

impl Ty {
    /// Get nominal type info if this is a struct/enum/protocol/type alias
    pub fn as_nominal(&self) -> Option<NominalTypeRef<'_>> {
        match self.kind() {
            TyKind::Nominal { symbol_id, substitutions } => {
                Some(NominalTypeRef { symbol_id: *symbol_id, substitutions })
            }
            _ => None,
        }
    }

    /// Helper to get the symbol ID for any nominal type
    pub fn symbol_id(&self) -> Option<SymbolId> {
        self.as_nominal().map(|n| n.symbol_id)
    }

    /// Helper to get substitutions for any nominal type
    pub fn substitutions(&self) -> Option<&Substitutions> {
        self.as_nominal().map(|n| n.substitutions)
    }
}
```

**Files affected:** ~15 files with TyKind match arms
**Lines saved:** ~500 lines

### 2. Type Transformation: Unified Framework

**Current:** Multiple similar implementations

- `Ty::substitute_self` (~115 lines)
- `Substitutions::apply` (~80 lines)
- `substitute_type` in utils.rs (~60 lines, incomplete)
- `Ty::is_specialization_of` (~100 lines)
- `Ty::overlaps_with` (~90 lines)

**Proposed:** Single transformation trait

```rust
pub trait TypeTransformer {
    fn transform(&mut self, ty: &Ty) -> Ty {
        self.transform_kind(ty)
    }

    fn transform_kind(&mut self, ty: &Ty) -> Ty {
        match ty.kind() {
            TyKind::SelfType => self.transform_self(ty),
            TyKind::TypeParameter(p) => self.transform_type_param(ty, p),
            TyKind::AssociatedType { .. } => self.transform_associated_type(ty),
            _ => self.transform_structural(ty),
        }
    }

    fn transform_self(&mut self, ty: &Ty) -> Ty { ty.clone() }
    fn transform_type_param(&mut self, ty: &Ty, _: SymbolId) -> Ty { ty.clone() }
    fn transform_associated_type(&mut self, ty: &Ty) -> Ty { ty.clone() }

    fn transform_structural(&mut self, ty: &Ty) -> Ty {
        ty.map_children(|child| self.transform(child))
    }
}

impl Ty {
    /// Map over all child types (unified for all type kinds)
    pub fn map_children(&self, mut f: impl FnMut(&Ty) -> Ty) -> Ty {
        match self.kind() {
            TyKind::Nominal { symbol_id, substitutions } => {
                let new_subs = substitutions.map_types(&mut f);
                Ty::nominal(*symbol_id, new_subs, self.span().clone())
            }
            TyKind::Tuple(elements) => {
                Ty::tuple(elements.iter().map(&mut f).collect(), self.span().clone())
            }
            TyKind::Array(elem) => Ty::array(f(elem), self.span().clone()),
            TyKind::Pointer(elem) => Ty::pointer(f(elem), self.span().clone()),
            TyKind::Function { params, return_type } => {
                Ty::function(
                    params.iter().map(&mut f).collect(),
                    f(return_type),
                    self.span().clone()
                )
            }
            _ => self.clone(),
        }
    }
}
```

**Lines saved:** ~300 lines

### 3. Conformance Analyzer: Unify Struct/Enum

**Current:** Separate functions for struct and enum

| Function | Lines | Duplicate |
|----------|-------|-----------|
| `check_struct_conformance` | ~130 | Yes |
| `check_enum_conformance` | ~130 | Yes |
| `link_protocol_methods_for_struct` | ~120 | Yes |
| `link_protocol_methods_for_enum` | ~120 | Yes |
| `resolve_protocol_type_for_link` | ~50 | Yes |
| `resolve_protocol_type_for_link_enum` | ~50 | Yes |

**Proposed:** Single unified function

```rust
/// Check conformance for any type container (struct or enum)
fn check_type_conformance(
    symbol_id: SymbolId,
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let type_name = model.query(SymbolNameFor { symbol_id }).unwrap();
    let conformances = model.query(AllConformancesFor { symbol_id });
    let associated_bindings = model.query(AssociatedTypeBindingsFor { symbol_id });
    let all_methods = model.query(MethodsFor { symbol_id });

    // ONE code path for both struct and enum
    for conformance_ty in &conformances {
        check_protocol_requirements(
            symbol_id,
            &type_name,
            conformance_ty,
            &associated_bindings,
            &all_methods,
            model,
            ctx,
        );
    }
}
```

**Lines saved:** ~400 lines

### 4. Binder Utils: Shared Copy Semantics

**Current:** Nearly identical code in `struct.rs` and `enum_binder.rs`

```rust
// struct.rs: compute_copy_semantics (~150 lines)
// enum_binder.rs: compute_copy_semantics (~150 lines)
```

**Proposed:** Extract to shared module

```rust
// binders/utils/copy_semantics.rs
pub fn compute_copy_semantics_for_type(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    field_types: impl Iterator<Item = Ty>,
    context: &mut BindingContext,
) -> CopySemantics {
    // ONE implementation
}
```

**Lines saved:** ~150 lines

### 5. Symbol Methods: Remove Delegation Boilerplate

**Current:** 7 symbol types each implement the same delegation

```rust
// In StructSymbol, EnumSymbol, ProtocolSymbol, FunctionSymbol, etc.
impl StructSymbol {
    pub fn type_parameters(&self) -> Vec<Arc<TypeParameterSymbol>> {
        self.metadata()
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.type_parameters().to_vec())
            .unwrap_or_default()
    }
    // ... same for is_generic(), where_clause()
}
```

**Proposed:** Query-based or trait-based

```rust
// Option A: Query
pub fn type_parameters_for(model: &SemanticModel, symbol_id: SymbolId) -> Vec<Arc<TypeParameterSymbol>> {
    model.query(GenericsFor { symbol_id })
        .map(|g| g.type_parameters)
        .unwrap_or_default()
}

// Option B: Extension trait on Symbol
pub trait HasGenerics: Symbol<KestrelLanguage> {
    fn type_parameters(&self) -> Vec<Arc<TypeParameterSymbol>> {
        self.metadata()
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.type_parameters().to_vec())
            .unwrap_or_default()
    }
    // ...
}

impl<T: Symbol<KestrelLanguage>> HasGenerics for T {}
```

**Lines saved:** ~150 lines

---

## TyKind and ExprKind Cleanup

### TyKind: Variants to Remove/Change

| Variant | Action | Reason |
|---------|--------|--------|
| `Struct`, `Enum`, `Protocol`, `TypeAlias` | Unify to `Nominal` | Structurally identical |
| `SelfType` | Consider removing | Should be substituted during binding |
| `UnresolvedFunction` | Keep | Needed for closure type inference |

### ExprKind: Variants to Remove

| Variant | Action | Reason |
|---------|--------|--------|
| `Grouping` | Remove | Just wraps inner expression, no semantic meaning |
| `PrimitiveMethodRef` | Remove | Only exists to error; catch at binding time |
| `LangIntrinsicRef` | Remove | Only exists to error; catch at binding time |

### ExprKind: Variants to Keep (Need Type Inference)

| Variant | Why Needed |
|---------|------------|
| `DeferredMethodCall` | Receiver type unknown until inference |
| `DeferredStaticCall` | Target type has inference variables |
| `ImplicitMemberAccess` | Expected type unknown until inference |
| `OverloadedRef` | Need argument types to disambiguate |
| `MethodRef` (multiple candidates) | Need argument types to disambiguate |

---

## Migration Path

### Phase 1: Add Unified Accessors (Non-Breaking)

1. Add `Ty::as_nominal()` method
2. Add `Ty::symbol_id()` and `Ty::substitutions()` helpers
3. Add `Ty::map_children()` method
4. Keep existing TyKind variants working

### Phase 2: Fix Missing Query Support

1. Add `Enum` case to `GenericsDataFor` query
2. Create unified `AssociatedTypeBindingsFor` query
3. Fix `conforms_to` to use unified approach

### Phase 3: Migrate TypeOracle Callers

1. Replace `match ty.kind() { TyKind::Struct => ... TyKind::Enum => ... }` patterns
2. Use `if let Some(nominal) = ty.as_nominal() { ... }` instead

### Phase 4: Extract Shared Binder Logic

1. Create `binders/utils/copy_semantics.rs`
2. Create `binders/utils/conformance_validation.rs`
3. Refactor `StructBinder` and `EnumBinder` to use shared code

### Phase 5: Unify Conformance Analyzer

1. Create `check_type_conformance` unified function
2. Create `link_protocol_methods` unified function
3. Remove struct/enum-specific versions

### Phase 6: Convert to Query System

1. Add caching layer to existing queries
2. Convert `add_behavior` calls to query-based resolution
3. Track query dependencies for incremental compilation

### Phase 7: Remove Extraneous ExprKind Variants

1. Remove `Grouping` - eliminate during parsing
2. Remove `PrimitiveMethodRef` - error at binding time
3. Remove `LangIntrinsicRef` - error at binding time

### Phase 8 (Long-term): Full ECS Migration

1. Introduce `EntityId` from syntax tree
2. Convert behaviors to cached queries
3. Convert analyzers to component-based systems
4. Remove `Symbol` types, use `EntityId + components`

---

## Summary

| Area | Current Lines | After Refactor | Savings |
|------|---------------|----------------|---------|
| TyKind match arms | ~500 | ~100 | ~400 |
| Type transformation | ~450 | ~150 | ~300 |
| Conformance analyzer | ~600 | ~200 | ~400 |
| Copy semantics | ~300 | ~150 | ~150 |
| Symbol delegation | ~200 | ~50 | ~150 |
| **Total** | ~2050 | ~650 | **~1400** |

The key insight: **behavior should be attached to components, not entity types**. The current code has `if struct { ... } else if enum { ... }` everywhere. True hECS would be: "run CopySemanticSystem on all entities with field-like children." One implementation, many entity types.
