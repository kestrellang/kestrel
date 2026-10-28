# Kestrel Query System Report

*Generated 2026-06-09. Survey of all `QueryFn` implementations, their call graph, inefficiencies, duplication, code smells, and documentation drift.*

43 queries implement `QueryFn` across 8 crates: kestrel-compiler (3), kestrel-hir-lower (5), kestrel-type-infer (3), kestrel-analyze (1), kestrel-mir-lower (1), kestrel-name-res (21), kestrel-semantics (10, one dead).

## 1. Query call tree

It's a DAG, drawn top-down from the pipeline entry points. `→` edges are `ctx.query(...)` calls reachable from the query's `execute` (including through helpers); ⟲ marks calls made inside loops.

```
ParseFile ──→ LexFile

InferWithDiagnostics (driver entry, converts errors→diagnostics)
└─ InferBody
   ├─ LowerBody
   │  ├─ ResolveValuePath ⟲ (desugar: operators, paths)
   │  ├─ ResolveTypePath   (via lower_ast_type)
   │  └─ ResolveBuiltin
   ├─ LowerCallableTypes ──→ ResolveTypePath
   ├─ LowerCallableReturnType ──→ LowerTypeAnnotation ──→ ResolveTypePath
   ├─ LowerExtensionTargetTypeArgs ──→ ExtensionTargetEntity
   ├─ WhereClausesOf ──→ ResolveTypePath, ResolveBuiltin   (×5 distinct entities per body)
   └─ [solver/resolve loops] ⟲
      ├─ TypeMembersByName, ConformingProtocols, ConformingProtocolInstantiations
      ├─ ResolveBuiltin, IsBuiltinProtocol, ResolveTypePath
      └─ NominalCopySemantics, TypeParamCopyRequirement, ConditionalCopyableParams

Analyze (dispatcher: one analyzer × one entity)
└─ body checks: LowerBody + InferBody; analyzers then query ~24 distinct queries

ClosureCaptures ──→ LowerBody, InferBody          (1 caller: mir-lower)
IsProtocolMethod (mir-lower)                       (1 caller: mir-lower itself)

── name-res layer ──────────────────────────────────────────
ResolveName ──→ ScopeFor, VisibleChildrenByName ⟲ (per wildcard import)
ResolveTypePath ──→ ResolveName, VisibleChildrenByName ⟲ (per segment)
ResolveValuePath ──→ VisibleChildrenByName ⟲, ConformingProtocols,
                     ExtensionsFor ⟲ (per protocol), IsVisibleFrom
ScopeFor ──→ ResolveModulePath, StdModules        (Arc<Scope> output ✓)
TypeMembers ──→ ExtensionsFor ⟲, ConformingProtocols ⟲
TypeMembersByName ──→ TypeMembers, IsVisibleFrom ⟲ (per member)
ProtocolMembers ──→ ExtensionsFor ⟲, ConformingProtocols
ProtocolMembersByName ──→ ProtocolMembers, IsVisibleFrom ⟲
ProtocolAssociatedTypes ──→ (same traversal as ProtocolMembers)
ConformingProtocols / ConformingProtocolInstantiations
   ──→ ExtensionsFor ⟲ (per discovered protocol) ──→ ExtensionTargetEntity ──→ ResolveTypePath
VisibleChildrenByName ──→ IsVisibleFrom ⟲ (per child)
ResolveBuiltin ──→ BuiltinIndex (Arc ✓, full-tree scan) ──→ EntityBuiltin

── semantics layer ─────────────────────────────────────────
ResolvedConformances ──→ ResolveTypePath
ProtocolRefines ──→ ConformingProtocols
ProtocolAllowsNegativeConformance ──→ EntityBuiltin
ExplicitlyNegatesProtocol ──→ ResolvedConformances
ExplicitlyConformsToProtocol ──→ ConformingProtocols
IsBuiltinProtocol ──→ ResolveBuiltin
TypeParamCopyRequirement ──→ ResolveBuiltin ×2, ProtocolRefines ⟲ (walks parent chain)
ConditionalCopyableParams ──→ NominalCopySemantics, ResolveBuiltin,
                              ConformingProtocolInstantiations,
                              ProtocolRefines ⟲, LowerExtensionTargetTypeArgs
NominalCopySemantics ──→ ResolveBuiltin ×2, ExplicitlyNegatesProtocol,
                         ExplicitlyConformsToProtocol,
                         LowerTypeAnnotation/LowerCallableTypes ⟲ (per field/case)
NominalTypeConformsToProtocol ──→ IsBuiltinProtocol ×2, ExplicitlyConformsToProtocol  ☠ DEAD
```

The biggest fan-in nodes: `ResolveTypePath` (~72 call sites repo-wide), `LowerBody` (~44), `InferBody` (~27), `ConformingProtocols` (~27), `ResolveBuiltin` (~26), `ExtensionsFor` (~25).

## 2. Inefficiencies in the tree

**A. Cache hits deep-clone outputs, and the two largest outputs aren't Arc-wrapped.** `kestrel-hecs/src/query.rs:320` clones `memo.value` (and `memo.deps`) on every hit. `ScopeFor` and `BuiltinIndex` already return `Arc<...>` precisely for this reason, but `LowerBody` returns `Option<HirBody>` (full expr/pat/stmt arenas) and `InferBody` returns `Option<TypedBody>` (6 HashMaps). Consequences compound downstream:

- The `Analyze` dispatcher (`kestrel-analyze/src/lib.rs:134,140`) calls `LowerBody` + `InferBody` per analyzer × per body — every one of those is a full clone of both structures.
- LSP `references.rs:57-136` walks every body in the workspace, cloning `HirBody` + `TypedBody` per body, then uses only `typed.resolutions`.

This is the single highest-leverage fix in the whole system: `Arc`-wrap the outputs of `LowerBody`, `InferBody` (and probably `ClosureCaptures`), following the existing `ScopeFor` precedent.

**B. The `*ByName` pattern is quadratic-ish per name.** `TypeMembersByName` and `ProtocolMembersByName` each pull the *full* member list and filter, calling `IsVisibleFrom` per member — every distinct name looked up on the same type re-clones the full `TypeMembers` vec and re-runs the visibility filter. Same shape in `VisibleChildrenByName` (`visibility.rs:117-126`), which is itself called in loops (per path segment in `ResolveTypePath`/`ResolveValuePath`, per wildcard import in `ResolveName`). Memoization keeps it from being catastrophic, but a name-indexed map output (`HashMap<String, Vec<Member>>`, Arc-wrapped, computed once per type) would replace the per-name re-filtering with one lookup.

**C. Dead and near-dead queries.**

- `NominalTypeConformsToProtocol` (`kestrel-semantics/src/lib.rs:519-560`) is **dead code** — its only reference outside the definition is its own `debug_trace!` string. Its function-form twin `hir_type_conforms_to_protocol` is what everyone actually uses. Delete it.
- Single-caller queries that may not pay for their memoization slot: `IsProtocolMethod` (1 site, mir-lower), `ProtocolAllowsNegativeConformance` (1 site), `ClosureCaptures` (1 site), `ResolvedConformances` (1 external site, `conformance_rules.rs:107` — though `ExplicitlyNegatesProtocol` also consumes it internally, so it's fine to keep).

**D. Three overlapping "does X conform to P" truths.** `ConformingProtocols` (transitive, extension-aware), `ResolvedConformances` (raw AST decls with polarity), and `ExplicitlyConformsToProtocol` (a thin `.contains()` over ConformingProtocols). The layering is defensible but undocumented — and `ExplicitlyConformsToProtocol` is misnamed: it includes extension-derived conformances via `ConformingProtocols`, which is not what "explicitly" implies. Callers must guess which to use; the call census shows they mostly guessed `ConformingProtocols` (27 vs 2).

**E. `hir_type_copy_semantics` / `hir_type_conforms_to_protocol` are query-heavy functions hiding behind a pure-function facade.** They're called from solver fixpoint loops and MIR, and each call re-runs `IsBuiltinProtocol` ×2, `TypeParamCopyRequirement`, etc. The sub-queries are memoized, but the wrappers re-execute their own glue (parent-chain walks, where-clause scans in `type_param_has_bound`, `kestrel-semantics/src/lib.rs:865-899`) on every invocation. These are the most natural queryification candidates in the codebase — the obstacle is keying on `HirTy`, which would need to be hashable/stable.

**F. Smaller loop hotspots.**

- `ConditionalCopyableParams` (`kestrel-semantics/src/lib.rs:413-450`) evaluates a `refines_copyable` closure (each call = a `ProtocolRefines` query) inside `.find(...any(...))`, twice over.
- `ConformingProtocolInstantiations` fires `ExtensionsFor` per newly discovered protocol in its closure loop (`conformances.rs:127-153`).
- `TypeParamCopyRequirement` walks the full parent chain re-scanning where-clauses per context (`kestrel-semantics/src/lib.rs:292-354`).
- ~70 `ResolveTypePath` call sites each allocate a fresh `Vec<String>` of segment names as the query key.

One claim from the sweep that was checked and **rejected**: `StdModules` "rescanning std per module" — it's keyed only by `root`, so it computes once and cache-hits after that. Its real (minor) cost is broad invalidation: any change under `std` re-runs the subtree walk.

## 3. Duplicated code — queryify or extract helpers

| Duplication | Locations | Suggested fix |
|---|---|---|
| `member_name_matches` defined twice, near-identical (the `init`/`subscript` sentinel logic) | `protocol_members.rs:131`, `type_members.rs:199` | One shared helper in `helpers.rs` |
| `find_assoc_type` defined twice, identical | `resolve_name.rs:286`, `resolve_type.rs:604` | Share the `pub(crate)` one |
| `TypeMembers` vs `ProtocolMembers` traversal — same 3-step walk (direct children → extension children → conformed/inherited-protocol extension children) implemented separately | `type_members.rs:120-179` vs `protocol_members.rs:147-196` | One generic member-traversal helper parameterized on the "parents" source |
| `TypeMembersByName` vs `ProtocolMembersByName` — byte-for-byte identical filter bodies | `type_members.rs:99-115` vs `protocol_members.rs:102-122` | Falls out of the above |
| "Walk extensions of X looking for name" boilerplate ×3 in resolve_value | `resolve_value.rs:344-366, 379-400, 555-578` | Extract helper; also collapses nesting in `walk_path_from` |
| Manual `children_of()` + `Name` filter walks in analyzers (~15 sites) re-implementing what `VisibleChildrenByName`/`TypeMembers` already compute | `conformance_completeness.rs` (8 sites), `extension_conflict.rs:194`, the four `*_cycles.rs` checkers, `type_annotation_resolution.rs:79`, `unknown_attribute.rs:89` | Audit each: some legitimately want *all* children regardless of visibility, but they should at least share one `children_named(cx, parent, name)` helper instead of 15 inline loops |

## 4. Code smells

- **Long, deeply nested execute paths:** `ResolveValuePath::walk_path_from` (~180 lines, `resolve_value.rs:268-446`), `ResolveTypePath::execute` (~120 lines), `create_param_types` in type-infer (~130 lines — the extension-self-type block at `kestrel-type-infer/src/lib.rs:140-204` wants extraction), `emit_method_where_clauses` (~115 lines, three constraint kinds in one loop).
- **Thread-local cycle detection** in `NominalCopySemantics` (`COMPUTING_COPY_SEMANTICS`, `kestrel-semantics/src/lib.rs:476-505`) — a side-channel working around the framework's panic-on-reentry. It works, but it's invisible to the dependency tracker and a trap for the next recursive query someone writes. Worth either promoting to a framework-level fixpoint/cycle-recovery mechanism or documenting loudly.
- **String-name fallback for `Copyable`** in `nominal_copy_semantics_impl` (`kestrel-semantics/src/lib.rs:770-782`) — a second, name-based path to non-copyability that exists for stdlib-less fixtures. Two sources of truth for one decision; deserves at least a comment fencing it to tests.
- **Misleading names:** `ExplicitlyConformsToProtocol` (see §2D); `LowerCallableReturnType` is a pure thin wrapper over `LowerTypeAnnotation` (fine, but it reads like an independent computation).
- **Inconsistent abstraction level in name-res:** `helpers.rs:38` defines `find_children_by_name` but several sites in the same crate inline the identical walk.
- **`Analyze` keyed by `analyzer: String`** — a stringly-typed query key; a typo'd analyzer ID silently dispatches nothing. An enum or interned ID would fail loudly and hash cheaper.

## 5. Documentation drift

The docs cover the core spine accurately (`LowerBody`/`InferBody`/`WhereClausesOf`/`Analyze` key shapes, the `WhereClausesOf` declaring-scope invariant in `type-inference.md:140`, the LexFile→ParseFile chain, the hecs glossary), but the per-crate query lists lag the code by roughly half, and three claims are outright wrong.

**Wrong claims:**

- `docs/contributing/architecture.md:128` — says kestrel-mir-lower provides a "`LowerMir` query". No such query exists; MIR lowering is plain functions (`lower_to_mir`), and the crate's only query is `IsProtocolMethod` (mentioned nowhere). mir-lower is also the only query-bearing crate with no `docs/architecture.md`.
- `docs/contributing/architecture.md:124` — credits kestrel-semantics with "conformance, witness resolution". There is zero witness code in kestrel-semantics; witness lowering lives in mir-lower/codegen.
- `docs/contributing/patterns.md:27-44` — the canonical query-authoring template declares `type Output = HirBody` then uses `?` in the body (wouldn't compile); the real output is `Option<HirBody>`. `quick-reference.md:163-164` repeats the unwrapped types (`HirBody`, `TypedBody`).

**Coverage gaps.** `quick-reference.md:168` defers to "each crate's `docs/architecture.md` for the full query list", but those lists cover only 22 of the 43 queries:

| Crate doc | Documented | Missing |
|---|---|---|
| name-res | 9 of 21 | All of `conformances.rs`, `protocol_members.rs`, `type_members.rs`, `resolve_builtin.rs` (10 queries) + `StdModules`. These four source files are also absent from the doc's "Source Files" tree — the doc predates them. |
| semantics | 4 of 10 | `ProtocolAllowsNegativeConformance`, `ExplicitlyNegatesProtocol`, `ExplicitlyConformsToProtocol`, `ConditionalCopyableParams`, `IsBuiltinProtocol` (+ the dead `NominalTypeConformsToProtocol`) |
| hir-lower | 3 of 5 | `LowerCallableReturnType`, `LowerExtensionTargetTypeArgs` |
| type-infer | 1 of 3 | `ClosureCaptures`, `WhereClausesOf` (documented in `docs/contributing/type-inference.md` but not the crate doc) |
| compiler | 2 of 3 | Its own `InferWithDiagnostics` |
| mir-lower | — | No `docs/architecture.md`; `IsProtocolMethod` undocumented |

The undocumented queries are precisely the newest layers: the copy-semantics cluster in semantics and the unified member/conformance discovery in name-res. The docs froze at the older resolution-only picture of name-res (scope/visibility/resolution/extensions) and the four-query picture of semantics.

**Sequencing note:** fixing the doc lists should come *after* deciding on §2C (delete the dead query), §2D (rename/collapse `ExplicitlyConformsToProtocol`), and §3 (traversal unification) — otherwise the lists get rewritten twice.

## Top 5 by leverage

1. `Arc`-wrap `LowerBody`/`InferBody` outputs (framework already clones per hit; `ScopeFor` shows the pattern). Biggest win for analyze + LSP.
2. Delete `NominalTypeConformsToProtocol`.
3. Unify the `TypeMembers`/`ProtocolMembers` traversal + `ByName` filtering, and consider name-indexed outputs to kill the per-name re-filter.
4. Extract the duplicated helpers (`member_name_matches`, `find_assoc_type`, extension-walk, analyzer child-walks).
5. Document (or collapse) the three-tier conformance model and the three-layer copy-semantics model — both are correct but only navigable with tribal knowledge today.
