# Migration Plan

Phased migration from kestrel-mir-2 to kestrel-mir-3.

## Strategy

New crate (`kestrel-mir-3`) alongside existing `kestrel-mir-2`. The live
compiler continues using mir-2 during development. Once mir-3 is ready,
switch the pipeline.

## Phase 1: IR Foundation + Verifier

**Goal**: kestrel-mir-3 crate with core types, builder, display, and
the OSSA verifier. No connection to the live compiler. Tested with
builder-constructed bodies.

**Creates**:
- `kestrel-mir-3/src/value.rs` — ValueId, ValueDef, Ownership
- `kestrel-mir-3/src/inst.rs` — InstKind, Instruction
- `kestrel-mir-3/src/block.rs` — BasicBlock, BlockParam
- `kestrel-mir-3/src/terminator.rs` — TerminatorKind, SwitchArm
- `kestrel-mir-3/src/body.rs` — OssaBody
- `kestrel-mir-3/src/builder.rs` — Test builder API
- `kestrel-mir-3/src/display.rs` — Pretty-printing
- `kestrel-mir-3/src/verify.rs` — OSSA ownership verifier

**Reuses from kestrel-mir-2** (shared leaf types):
- `TyId`, `TyArena`, `MirTy` — type system
- `Op`, `IntBits`, `FloatBits`, `Signedness` — operations
- `Immediate`, `ImmediateKind` — constants
- `FieldIdx`, `VariantIdx`, `BlockId` — indices
- `CopyBehavior`, `DropBehavior`, `TypeInfo` — type metadata
- `StructDef`, `EnumDef`, `ProtocolDef`, `WitnessDef` — item definitions
- `WitnessMethodKey` — witness method keys
- `ParamConvention` — parameter conventions
- `SwitchCase` — switch patterns

Does not reuse as-is:
- `FunctionDef` — MIR-2 stores `body: Option<MirBody>`.
- `MirModule` — contains `Vec<FunctionDef>`.
- `Callee` — conceptually similar, but `Thin`/`Thick` use `ValueId`
  instead of MIR-2 `Place`.

Approach: make `FunctionDef` and `MirModule` generic over the body type
(`FunctionDef<B>` / `MirModule<B>`). The generic param appears only in
`body: Option<B>`. MIR-2 uses `FunctionDef<MirBody>`, MIR-3 uses
`FunctionDef<OssaBody>`. Shared metadata (name, params, kind, etc.)
remains one source of truth.

**Tests**: ~150 builder tests verifying:
- Value lifecycle (CopyValue, MoveValue, DestroyValue)
- Borrow scoping (BeginBorrow, EndBorrow pairing)
- Block arguments (Jump/Branch/Switch with args, block params)
- Forwarding (Struct/Tuple/Enum construction + extraction)
- Address state (`Load`, `CopyAddr`, `Take`, `StoreInit`, `StoreAssign`, `DestroyAddr`)
- Sub-field address state (`Uninit` + `FieldAddr` + per-field init tracking,
  double-init rejection, partial-init Take rejection, failable init cleanup)
- Verifier catches: unconsumed owned, double-consume, use-after-consume,
  borrow escape, missing block args, invalid address state, sub-field init
  violations, trivial type violations

**Estimate**: ~3,000 lines. ~1 week.

## Phase 2: OSSA Lowering

**Goal**: New lowering crate that emits OSSA from HIR. The hardest
phase — must emit correct copy_value, destroy_value, block arguments.

**Creates**:
- `kestrel-mir-lower-3/src/body_lower.rs` — HIR → OSSA body lowering
- `kestrel-mir-lower-3/src/scope.rs` — Scope frame stack for destroy tracking
- `kestrel-mir-lower-3/src/control_flow.rs` — Block arg threading helpers

**Approach**: Adapt the existing body_lower.rs (5,499 lines). The
architecture is clean and modular (see lowering.md for analysis).
Key changes at the major lowering choke points:

1. `emit_value_transfer` → emit CopyValue/MoveValue or address CopyAddr/Take
2. `arg_for_value` → emit BeginBorrow/BeginBorrowAddr around calls
3. stored-place assignment → emit StoreInit/StoreAssign
4. `lower_if` → thread complete owned live-ins through merge
5. `lower_loop` → thread complete owned live-ins through header
6. `lower_match` → thread complete owned live-ins through join
7. Scope exits → emit DestroyValue/DestroyAddr

**Tests**: Run existing testdata through the new pipeline:
- Compile `.ks` test files → OSSA → verify
- All 71 deinit tests should pass (verifier catches leaks)
- All 34 copy_semantics tests should pass
- All 24 cloneable tests should pass

**Estimate**: ~5,000 lines (mostly adapted from existing lowerer). ~2 weeks.

## Phase 3: Passes

**Goal**: Port drop_shim, thunk, layout, monomorphization to OSSA.

**Creates**:
- `kestrel-mir-3/src/passes/drop_shim.rs` — __drop$T in OSSA form
- `kestrel-mir-3/src/passes/thunk.rs` — Thunk wrappers in OSSA form
- `kestrel-mir-3/src/passes/layout.rs` — Unchanged (layout reads types, not instructions)
- `kestrel-mir-3/src/mono/` — Monomorphization operating on OSSA bodies

**Eliminated**: clone_elab, drop_elab, drop_flag_expand, init_state,
liveness, drop_fix. These passes do not exist in mir-3.

**Estimate**: ~2,000 lines. ~1 week.

## Phase 4: Codegen

**Goal**: Cranelift codegen consuming OSSA.

**Creates**:
- `kestrel-codegen-cranelift-3/` — New codegen crate

**Key changes from current codegen**:
1. Block argument threading in all terminators
2. InstKind match instead of StatementKind
3. CopyValue → clone witness call (or use pre-codegen copy lowering pass)
4. DestroyValue → drop shim call
5. BeginBorrow/BeginBorrowAddr/EndBorrow → take_address/address / no-op
6. CopyAddr/Take/StoreInit/StoreAssign/DestroyAddr → explicit memory ops
7. ValueId → Cranelift Value mapping instead of LocalId → Variable

**Estimate**: ~3,000 lines (adapted from current codegen). ~1-2 weeks.

## Phase 5: Integration

**Goal**: Wire mir-3 into the compiler pipeline, retire mir-2.

**Changes**:
- `kestrel-compiler/src/lib.rs` — Switch pipeline from mir-2 to mir-3
- Remove dependency on kestrel-mir-2, kestrel-mir-lower-2,
  kestrel-codegen-cranelift-2

**Verification**:
- Full triage run — all 17K+ tests must pass
- Perch app — verify the 8KB/req leak is fixed
- libgmalloc validation — no use-after-free

**Estimate**: ~1 day wiring + 2-3 days debugging integration issues.

## Timeline Summary

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1. IR Foundation | Types, builder, verifier | 1 week |
| 2. Lowering | HIR → OSSA | 2 weeks |
| 3. Passes | drop_shim, thunk, mono | 1 week |
| 4. Codegen | Cranelift backend | 1-2 weeks |
| 5. Integration | Wire in, retire mir-2 | 3-5 days |
| **Total** | | **~6-8 weeks** |

## Risk Mitigation

### Risk: Lowerer emits incorrect OSSA

**Mitigation**: The OSSA verifier runs after every pass. If the lowerer
produces an unconsumed @owned value, missing block args, or borrow
violations, the verifier catches it immediately with a specific error
message pointing to the instruction and block.

### Risk: Block arguments break existing patterns

**Mitigation**: The lowerer can start conservative — emit CopyValue for
every Clone use, emit DestroyValue for every owned value at every scope
exit. The copy_optimize pass eliminates unnecessary copies later.
Correctness first, optimization second.

### Risk: Codegen regression

**Mitigation**: Run against the same testdata. The execution tests
verify runtime behavior, not IR shape. If the OSSA codegen produces
the same native code semantics as MIR-2 codegen, all execution tests
pass regardless of IR format changes.

### Risk: Performance regression from extra copies

**Mitigation**: The copy_optimize pass eliminates copies where the
source is immediately destroyed. For well-written code, the optimized
OSSA should produce the same number of clone calls as MIR-2's clone_elab.
For pathological cases, it may produce more — but correctness is more
important than performance, and the extra copies can be optimized later.

## What's NOT in Scope

- Borrow checker / lexical lifetimes / NLL — not needed today (borrows are
  call-scoped). But the IR and verifier accept @guaranteed block params with
  borrow provenance tracking, so adding lexical lifetimes later is a lowerer
  change, not an IR redesign. The foundation (provenance on ValueDef,
  cross-block borrow tracking in the verifier) is in place from day one.
- begin_access / end_access (exclusivity) — Kestrel is single-threaded.
- Lexical lifetimes — Kestrel has no implicit copies to suppress.
- Partial move optimization — the lowerer can use whole-aggregate moves
  initially. DestructureStruct/field extraction for partial moves can be
  adopted incrementally.
- Closure borrow captures — closure environments always own or trivially
  copy their captures. Borrowed captures are materialized as ref temps
  (Pointer(T), which is @none) before ApplyPartial. This is consistent
  with the "borrows are call-scoped only" design. No @guaranteed captures.
- Array element extraction — arrays are always address-backed. Element
  access uses Take/CopyAddr on the address, not SSA-level extraction.
  No ArrayExtract instruction needed.
