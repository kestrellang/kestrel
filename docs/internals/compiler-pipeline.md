# Compiler Pipeline

Complete reference for the Kestrel compilation pipeline: every phase, every crate, every analyzer, and the full query dependency graph.

## Pipeline Overview

```
Source Code (.ks files)
       │
       ▼
┌──────────────────────────────────────────────────────┐
│  Phase 1: LEXING                   [kestrel-lexer]   │
│  lex(source, file_id) → Iterator<Spanned<Token>>     │
│  Library: Logos                                       │
└──────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────┐
│  Phase 2: PARSING                  [kestrel-parser]  │
│  Parser::parse(source, tokens) → ParseResult         │
│  Library: Chumsky                                    │
│  Architecture: Event-driven (→ EventSink → TreeBuilder) │
└──────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────┐
│  Phase 3: SYNTAX TREE          [kestrel-syntax-tree] │
│  TreeBuilder → SyntaxNode (lossless Rowan CST)       │
│  Preserves whitespace, comments, trivia              │
└──────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│  Phase 4: SEMANTIC ANALYSIS                                         │
│                                                                      │
│  4a. BUILD          [kestrel-semantic-tree-builder]                  │
│      SemanticModelBuilder::build() → SemanticModel                  │
│      Creates symbols, attaches behaviors                            │
│      ⛔ errors halt pipeline                                        │
│                         │                                            │
│                         ▼                                            │
│  4b. BIND           [kestrel-semantic-tree-binder]                  │
│      SemanticBinder::bind(model) → SemanticModel (enriched)         │
│      Resolves types, imports, bodies; emits deferred expressions    │
│      ⛔ errors halt pipeline                                        │
│                         │                                            │
│                         ▼                                            │
│  4c. VALIDATE       [kestrel-semantic-analyzers]                    │
│      run_all(analyzers, model, ctx) → Diagnostics                   │
│      Three sub-phases (see Analyzer Phases below)                   │
│      ⛔ errors between sub-phases halt pipeline                     │
└──────────────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────┐
│  Phase 5: MIR LOWERING  [kestrel-execution-graph-lowering] │
│  lower_module(model, module) → MirContext             │
│  Flat place-based IR: assign, call, drop, jump, branch │
│  Generates __kestrel_init_statics()                   │
└──────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────┐
│  Phase 6: CODE GENERATION   [kestrel-codegen-cranelift] │
│  1. monomorphize::collect_all() — BFS generic instances │
│  2. Define statics (data section)                     │
│  3. Declare functions (all signatures)                │
│  4. Define functions (MIR → Cranelift IR)             │
│  5. Link to executable (system linker)                │
└──────────────────────────────────────────────────────┘
```

## Data Flow Between Phases

| Phase | Input | Output | Key Type |
|-------|-------|--------|----------|
| 1 Lex | `&str` source | `Iterator<Spanned<Token>>` | `Token` |
| 2 Parse | Tokens + source | `ParseResult { tree, errors }` | `Event` |
| 3 Syntax | Events | `SyntaxNode` (Rowan CST) | `SyntaxNode` |
| 4a Build | `SyntaxNode` per file | `SemanticModel` | `Symbol<KestrelLanguage>` |
| 4b Bind | `SemanticModel` | `SemanticModel` (enriched) | resolved types + bodies |
| 4c Validate | `SemanticModel` | diagnostics only | `Analyzer` trait |
| 5 MIR | `SemanticModel` | `MirContext` | `Statement`, `Terminator`, `Place` |
| 6 Codegen | `MirContext` | object bytes → executable | Cranelift IR |

## Semantic Model Mutation Rules

| Phase | May Mutate | May NOT Mutate |
|-------|-----------|----------------|
| **BUILD** | Symbol creation, parent/child, syntax_map, sources | Type/value resolution |
| **BIND** | Derived info (types, signatures, bodies), registries | New symbols that change surface area |
| **VALIDATE** | Diagnostics only | Model or resolution results |

---

## Analyzer Phases (Phase 4c)

### Sub-Phase 1: Pre-Inference

Run before type inference. Errors here halt the pipeline.

| # | Analyzer | Purpose |
|---|----------|---------|
| 1 | `TypeAliasCycleAnalyzer` | Detect circular type alias dependencies |
| 2 | `StructCycleAnalyzer` | Detect circular struct field dependencies |
| 3 | `ConstraintCycleAnalyzer` | Detect circular constraint dependencies |
| 4 | `ConformanceAnalyzer` | Validate protocol conformance requirements |
| 5 | `ExtensionConflictAnalyzer` | Detect conflicting extension definitions |
| 6 | `FieldAnalyzer` | Validate struct/class field declarations |
| 7 | `InitializerVerificationAnalyzer` | Validate initializer definitions |
| 8 | `AssignmentValidationAnalyzer` | Validate assignment operations |
| 9 | `DefiniteAssignmentAnalyzer` | Ensure variables are assigned before use |
| 10 | `DeadCodeAnalyzer` | Detect unreachable code |
| 11 | `ExhaustiveReturnAnalyzer` | Ensure all code paths return |
| 12 | `GuardLetDivergenceAnalyzer` | Validate guard-let divergence |
| 13 | `ClosureAnalyzer` | Analyze closure structure (before inference rewrites) |

### Sub-Phase 2: Type Resolution

Type inference and pattern checking. Errors here halt the pipeline.

| # | Analyzer | Purpose |
|---|----------|---------|
| 1 | `TypeInferenceAnalyzer` | **Constraint-based type inference** (`kestrel-semantic-type-inference`) |
| 2 | `RefutablePatternAnalyzer` | Validate refutable patterns |
| 3 | `ForLoopPatternAnalyzer` | Validate for-loop patterns |
| 4 | `IrrefutablePatternAnalyzer` | Validate irrefutable patterns |
| 5 | `ExhaustivenessAnalyzer` | Ensure match/switch exhaustiveness (Maranget's algorithm) |
| 6 | `TypeCheckAnalyzer` | Validate type compatibility |
| 7 | `FunctionBodyAnalyzer` | Validate function body type consistency |
| 8 | `SubscriptValidationAnalyzer` | Validate subscript operations |

### Sub-Phase 3: Post-Checking

Final validators after all types are resolved.

| # | Analyzer | Purpose |
|---|----------|---------|
| 1 | `ProtocolMethodAnalyzer` | Validate protocol method conformance |
| 2 | `StaticContextAnalyzer` | Validate static context constraints |
| 3 | `DuplicateSymbolAnalyzer` | Detect duplicate symbol declarations |
| 4 | `DuplicateCallableAnalyzer` | Detect duplicate callable definitions |
| 5 | `DuplicateCaseAnalyzer` | Detect duplicate enum cases |
| 6 | `DuplicateLabelAnalyzer` | Detect duplicate labels |
| 7 | `RecursiveEnumAnalyzer` | Validate recursive enum definitions |
| 8 | `VisibilityConsistencyAnalyzer` | Validate visibility modifier consistency |
| 9 | `GenericsAnalyzer` | Validate generic type constraints |
| 10 | `ImportAnalyzer` | Validate import statements |

**Total: 31 analyzers across 3 sub-phases.**

---

## Query System

Queries are memoized pure functions over the `SemanticModel`. They form the primary API for reading semantic information.

### Infrastructure

```rust
// query.rs
pub trait Query: Hash + Eq + Clone + 'static {
    type Output: Clone;
    fn execute(self, model: &SemanticModel) -> Self::Output;
}

// model.rs — dispatch + caching
model.query(SomeQuery { ... }) → Q::Output
// Cache: HashMap<TypeId, HashMap<Q, Q::Output>> (type-erased, RefCell for reentrancy)
```

### Complete Query List

46 queries organized by dependency depth.

#### Level 0 — Foundational (no query dependencies)

| Query | Input | Output |
|-------|-------|--------|
| `SymbolFor` | `SymbolId` | `Option<Arc<dyn Symbol>>` |
| `ExtensionsFor` | `SymbolId` | `Vec<Arc<ExtensionSymbol>>` |

#### Level 1 — Direct symbol access

| Query | Input | Output | Depends On |
|-------|-------|--------|------------|
| `ConformancesForSymbol` | `SymbolId` | `Vec<Ty>` | `SymbolFor` |
| `ScopeFor` | `SymbolId` | `Arc<Scope>` | `SymbolFor` |
| `ImportsInScope` | `SymbolId` | `Vec<Arc<Import>>` | `SymbolFor` |
| `DeclaredNamesInScope` | `SymbolId` | `Vec<DeclaredName>` | `SymbolFor` |
| `ExecutableBodyFor` | `SymbolId` | `Option<CodeBlock>` | `SymbolFor` |
| `HasBody` | `SymbolId` | `Option<bool>` | `SymbolFor` |
| `LocalName` | `SymbolId`, `LocalId` | `Option<String>` | `SymbolFor` |
| `VisibilityLevelOf` | `SymbolId` | `VisibilityLevel` | `SymbolFor` |
| `ConcreteSelfType` | `SymbolId` | `Option<Ty>` | `SymbolFor` |
| `ResolvedAliasedType` | `SymbolId` | `Option<Ty>` | `SymbolFor` |
| `GenericsDataFor` | `SymbolId` | `Option<GenericsData>` | `SymbolFor` |
| `StructFields` | `SymbolId` | `Vec<StructFieldInfo>` | `SymbolFor` |
| `StructMethods` | `SymbolId` | `Vec<(String, Span)>` | `SymbolFor` |
| `FunctionsInSymbol` | `SymbolId` | `Vec<Arc<FunctionSymbol>>` | `SymbolFor` |
| `ExtensionMethods` | `SymbolId` | `Vec<(String, Span)>` | `SymbolFor` |
| `ChildByName` | `SymbolId`, `String` | `Option<Arc<dyn Symbol>>` | `SymbolFor` |
| `IsMarkerProtocol` | `SymbolId` | `bool` | `SymbolFor` |
| `ProtocolAssociatedTypesWithDefaults` | `SymbolId` | `HashMap<String, Option<SignatureType>>` | `SymbolFor` |
| `ResolveModulePath` | `Vec<String>`, `SymbolId` | `Result<SymbolId, Error>` | registry |

#### Level 2 — Composite queries

| Query | Input | Output | Depends On |
|-------|-------|--------|------------|
| `AncestorOfKind` | `SymbolId`, `Kind` | `Option<SymbolId>` | `SymbolFor` (recursive walk) |
| `IsInsideAny` | `SymbolId`, `Vec<Kind>` | `bool` | `SymbolFor` (recursive walk) |
| `IsVisibleFrom` | `SymbolId`, `SymbolId` | `bool` | `SymbolFor`, `AncestorOfKind` |
| `VisibleChildren` | `SymbolId`, `SymbolId` | `Vec<Arc<dyn Symbol>>` | `SymbolFor`, `IsVisibleFrom` |
| `VisibleChildrenByName` | `SymbolId`, `String`, `SymbolId` | `Vec<Arc<dyn Symbol>>` | `SymbolFor`, `IsVisibleFrom` |
| `AllConformancesFor` | `SymbolId` | `Vec<Ty>` | `ConformancesForSymbol`, `ExtensionsFor` |
| `AllMethodsFor` | `SymbolId` | `Vec<Arc<FunctionSymbol>>` | `FunctionsInSymbol`, `ExtensionsFor` |
| `AllInitializersFor` | `SymbolId` | `Vec<Arc<InitializerSymbol>>` | `ExtensionsFor`, `SymbolFor` |
| `StructFieldTypes` | `SymbolId` | `Vec<StructFieldTypeInfo>` | `StructFields` |
| `WhereClausesInScope` | `SymbolId` | `Vec<WhereClause>` | `SymbolFor` (parent chain walk) |
| `SelfProtocolBounds` | `SymbolId` | `Vec<SymbolId>` | `WhereClausesInScope`, `SymbolFor` |
| `TypeParameterBounds` | `SymbolId` | `Vec<Ty>` | `SymbolFor`, `ExtensionsFor` |
| `ExtensionBoundsForParam` | `SymbolId`, `SymbolId` | `Option<Vec<Ty>>` | `SymbolFor` |
| `AssociatedTypeBoundsInContext` | `SymbolId`, `Option<SymbolId>` | `Vec<Ty>` | `SymbolFor`, `WhereClausesInScope` |
| `ProtocolMethodsWithDefiner` | `SymbolId` | `Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)>` | `ConformancesForSymbol`, `SymbolFor` |
| `ProtocolInitializersWithDefiner` | `SymbolId` | `Vec<(Arc<ProtocolSymbol>, Arc<InitializerSymbol>)>` | `ConformancesForSymbol`, `SymbolFor` |
| `ProtocolRequiredMethods` | `SymbolId` | `Vec<(Signature, Arc<FunctionSymbol>)>` | `ExtensionsFor`, `SymbolFor` |
| `ProtocolRequiredInitializers` | `SymbolId` | `Vec<(Signature, Arc<InitializerSymbol>)>` | `ExtensionsFor`, `SymbolFor` |
| `ProtocolRequiredProperties` | `SymbolId` | `Vec<PropertyRequirement>` | `SymbolFor` |

#### Level 3 — Resolution queries

| Query | Input | Output | Depends On |
|-------|-------|--------|------------|
| `ResolveName` | `String`, `SymbolId` | `SymbolResolution` | `ScopeFor`, `ImportsInScope`, `ResolveModulePath`, `VisibleChildrenByName`, `SymbolFor`, `InheritedProtocolMember` |
| `ResolveTypePath` | `Vec<String>`, `SymbolId` | `TypePathResolution` | `ResolveName`, `SymbolFor`, `VisibleChildrenByName`, `InheritedProtocolMember` |
| `ResolveValuePath` | `Vec<String>`, `SymbolId` | `ValuePathResolution` | `ResolveName`, `SymbolFor`, `VisibleChildrenByName`, `IsVisibleFrom`, `ExtensionsFor`, `ResolvedAliasedType` |
| `InheritedProtocolMember` | `SymbolId`, `String` | `Option<SymbolId>` | `SymbolFor`, self-recursive |
| `AssociatedTypeBindingsFor` | `SymbolId` | `HashMap<String, SignatureType>` | `SymbolFor`, `ExtensionsFor`, `ResolvedAliasedType` |

#### Level 4 — Inference (highest level)

| Query | Input | Output | Depends On |
|-------|-------|--------|------------|
| `InferenceResultFor` | `SymbolId` | `Option<Solution>` | `SymbolFor`, entire `TypeOracle` API |

### Query Dependency Graph

```
                    ┌──────────────┐
                    │ SymbolFor    │  ◄── Foundation: nearly everything depends on this
                    └──────┬───────┘
                           │
          ┌────────────────┼──────────────────────────┐
          │                │                           │
          ▼                ▼                           ▼
   ┌─────────────┐  ┌──────────────┐          ┌──────────────┐
   │ ScopeFor    │  │ ExtensionsFor│          │ Conformances │
   │ ImportsIn   │  │              │          │ ForSymbol    │
   │ Scope       │  └──────┬───────┘          └──────┬───────┘
   └──────┬──────┘         │                         │
          │         ┌──────┴───────────────────┐     │
          │         │                          │     │
          ▼         ▼                          ▼     ▼
   ┌──────────┐  ┌──────────────┐      ┌───────────────────┐
   │ Resolve  │  │ AllMethodsFor│      │ ProtocolMethods   │
   │ Name     │  │ AllInitsFor  │      │ WithDefiner       │
   └────┬─────┘  │ AllConforms  │      │ ProtocolRequired* │
        │        └──────────────┘      └───────────────────┘
        │
   ┌────┴──────────────┐
   │                   │
   ▼                   ▼
┌───────────────┐  ┌────────────────┐
│ ResolveType   │  │ ResolveValue   │
│ Path          │  │ Path           │
└───────┬───────┘  └────────┬───────┘
        │                   │
        └─────────┬─────────┘
                  │
                  ▼
        ┌──────────────────┐
        │ InferenceResult  │  ◄── Top: uses TypeOracle which calls many queries
        │ For              │
        └──────────────────┘
```

### Visibility Chain

```
SymbolFor → AncestorOfKind → IsVisibleFrom → VisibleChildren / VisibleChildrenByName
```

### Name Resolution Chain

```
ScopeFor + ImportsInScope + ResolveModulePath
            │
            ▼
       ResolveName  ←── walks scope chain, checks imports, extensions, inherited members
            │
       ┌────┴────┐
       ▼         ▼
  ResolveType  ResolveValue
  Path         Path
```

### Where Clause / Bounds Chain

```
SymbolFor (parent chain walk) → WhereClausesInScope
                                     │
                    ┌────────────────┼────────────────┐
                    ▼                ▼                ▼
            SelfProtocol     AssociatedType    TypeParameter
            Bounds           BoundsInContext   Bounds
```

---

## Crate Dependency Map

```
kestrel (CLI binary)
  └─ kestrel-compiler
       ├─ kestrel-lexer
       ├─ kestrel-parser
       │   └─ kestrel-syntax-tree
       ├─ kestrel-semantic-tree-builder        (BUILD)
       │   └─ kestrel-semantic-tree
       ├─ kestrel-semantic-tree-binder         (BIND)
       │   └─ kestrel-semantic-tree
       ├─ kestrel-semantic-model               (queries + model)
       │   └─ kestrel-semantic-tree
       ├─ kestrel-semantic-analyzers           (VALIDATE)
       │   ├─ kestrel-semantic-type-inference
       │   └─ kestrel-semantic-pattern-matching
       ├─ kestrel-execution-graph-lowering     (MIR lowering)
       │   └─ kestrel-execution-graph          (MIR types)
       ├─ kestrel-codegen-cranelift            (native codegen)
       │   └─ kestrel-codegen                  (layout, mangling)
       ├─ kestrel-reporting                    (diagnostics)
       └─ kestrel-span                         (source locations)
```

## Compilation Entry Point

`lib/kestrel-compiler/src/compilation.rs`:

```
Compilation::from_sources(sources)
  ├─ For each source file:
  │  ├─ lex() → tokens
  │  ├─ Parser::parse() → SyntaxNode
  │  └─ builder.add_file(syntax_node)
  │
  ├─ ⛔ Stop if lex/parse errors
  │
  ├─ builder.build() → SemanticModel           (BUILD)
  ├─ ⛔ Stop if build errors
  │
  ├─ SemanticBinder::bind() → SemanticModel     (BIND)
  ├─ ⛔ Stop if bind errors
  │
  ├─ run_all(pre_inference_analyzers)            (VALIDATE phase 1)
  ├─ ⛔ Stop if errors
  ├─ run_all(type_resolution_analyzers)          (VALIDATE phase 2)
  ├─ ⛔ Stop if errors
  ├─ run_all(post_checking_analyzers)            (VALIDATE phase 3)
  │
  └─ Return Compilation { model, diagnostics }

compilation.build(target, options, output)
  ├─ lower_to_execution_graph() → MirContext
  ├─ compile(mir) → object bytes
  └─ link_executable(object, output)
```
