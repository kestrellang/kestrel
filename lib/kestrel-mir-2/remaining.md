# kestrel-mir-2: Remaining Work

## Done

- Foundation types (id, op, ty, place, operand, immediate, statement, terminator, body, item, layout, substitute, builder, display) — 179 tests
- Type queries (`ty_query.rs`: `copy_behavior`, `needs_drop`) — 12 tests
- Shared dataflow (`passes/dataflow.rs`: CfgInfo, forward/backward fixpoint) — 17 tests
- Backward liveness (`passes/liveness.rs`) — 13 tests
- Forward init-state (`passes/init_state.rs`) — 15 tests
- Clone elaboration (`passes/clone_elab.rs`) — 7 tests
- Drop shim synthesis (`passes/drop_shim.rs`) — 7 tests
- Drop elaboration (`passes/drop_elab.rs`) — 8 tests
- Non-generic layout (`passes/layout.rs`) — 10 tests
- Generic verifier (`passes/verify.rs`) — 13 tests
- Pipeline integration tests (`passes/mod.rs`) — 6 tests
- **Total: 288 tests, 10.9k lines**

## Remaining

### 1. Thunk synthesis (`passes/thunk.rs`)

Generate wrapper functions for `ApplyPartial` targets conforming to the thick-callable ABI: an ignored env pointer parameter followed by forwarded arguments.

- **Spec:** `docs/passes.md` section 1b ("Thunk synthesis")
- **Scope:** Scan bodies for `Rvalue::ApplyPartial { func }`, generate `FunctionKind::Thunk { original }` wrappers. Thunks are generic — they inherit the original function's type params.
- **Estimate:** ~200 lines impl + ~100 lines tests. Independent of CFG analysis.

### 2. Monomorphization (`mono/`)

Consumes generic `MirModule`, produces concrete `MonoModule`. The largest remaining piece.

- **Spec:** `docs/monomorphization.md` (320 lines), `docs/architecture.md` MonoModule section, `docs/ir.md` MonoCallee/MonoRvalue/MonoStatementKind
- **Modules per `docs/architecture.md`:**
  - `mono/types.rs` — MonoModule, MonoFunction, MonoBody, MonoCallee, MonoRvalue, MonoStatementKind, MonoStruct, MonoEnum, MonoStatic, MonoParam, MonoField
  - `mono/collect.rs` — Phase 1: BFS instantiation discovery from entry points
  - `mono/witness.rs` — witness pattern matching + resolution (`find_witness`)
  - `mono/mangle.rs` — v0 name mangling scheme (`docs/monomorphization.md` "Name mangling")
  - `mono/mod.rs` — `monomorphize()` entry point, Phase 2-5 orchestration

#### Phase breakdown

| Phase | What | Key operation |
|-------|------|---------------|
| 0 | Define mono types | `MonoModule`, `MonoFunction`, `MonoCallee`, etc. |
| 1 | Instantiation discovery | BFS from non-generic functions, resolve witnesses for reachability |
| 2 | Body monomorphization | Clone generic bodies, `substitute()` all types, resolve `Callee::Witness` |
| 3 | ID assignment + rewriting | Assign `MonoFuncId`, rewrite callees to `MonoCallee::Direct`, rewrite `FunctionRef` to `MonoFunctionRef` |
| 4 | Type/layout resolution | Build `MonoStruct`/`MonoEnum` with concrete fields and computed layouts |
| 5 | Assembly | Construct `MonoModule`, verify no generics remain |

- **Dependencies:** Uses `substitute.rs`, `layout.rs` (StructLayout arithmetic), `ty_query.rs`
- **Estimate:** ~1500-2000 lines impl + ~800-1000 lines tests

#### Suggested build order

1. `mono/types.rs` — define all mono types (testable: construction, equality)
2. `mono/mangle.rs` — name mangling (testable: input→output string tests)
3. `mono/witness.rs` — witness pattern matching (testable: match concrete types against witness patterns)
4. `mono/collect.rs` — instantiation discovery (testable: BFS on small modules)
5. `mono/mod.rs` — full monomorphize() (integration tests: generic module → MonoModule)

### 3. Mono verifier (`mono/verify.rs`)

Post-monomorphization verification.

- **Spec:** `docs/passes.md` section 6 ("Verify (mono)")
- **Checks:**
  - No `MirTy::TypeParam`, `SelfType`, or `AssociatedProjection` in any body or type
  - No `Callee::Witness` in any body
  - All `TypeInfo.layout` values are `Some` (fully computed)
  - All `MonoCallee::Direct` targets are valid `MonoFuncId`s
  - All `MonoFunction.body` is `Some` unless `extern_info` is `Some`
- **Estimate:** ~150 lines impl + ~100 lines tests. Pattern matching over MonoModule.
