# Design Rationale

Why each decision was made. Each section: old design → what broke → new
design → why. This is the "don't revert this" document.

## Operand / Rvalue split

**Old:** `Value` had 5 variants (Copy, Move, Ref, RefMut, Const). `Rvalue`
had the same 5 plus 8 compound variants. Every pass that processed operands
had to match both enums — ~15 parallel match sites across 6 files.

**What broke:** Adding a new ownership-related check required updating
both `verify_value()` and `verify_rvalue()`. Clone elaboration checked
`Rvalue::Copy(place)` at the statement level AND `Value::Copy(place)` inside
composites — same structural check, two code paths. Drop elaboration had
`kill_rvalue()` and `kill_value()` doing identical work.

**New:** `Operand` is mode-free (Place | Const). `Rvalue::Use(Operand, UseMode)`
is the bridge. Compound rvalues take `(Operand, UseMode)` pairs. Call args
take `(Operand, ArgMode)`. The five duplicated variants collapse into one.

**Why:** Rust MIR's `Operand` / `Rvalue::Use(Operand)` pattern eliminates
the duplication entirely. The `Use` bridge means every ownership decision
is expressed once, in one place. Passes iterate `rvalue.operands()` instead
of matching 13 variants.

**Reference:** Rust MIR `Operand` (rustc_middle::mir::Operand),
`Rvalue::Use(Operand)` (rustc_middle::mir::Rvalue).

## UseMode vs ArgMode

**Old:** One `Value` type with 4 ownership variants used everywhere — call
args, assignments, composite rvalue fields. No distinction between value
transfer and calling convention.

**New:** `UseMode { Copy, Move }` for value positions (assignments, compound
rvalue operands). `ArgMode { Copy, Move, Ref, RefMut }` for call sites only.

**Why:** Ref/RefMut on a struct field is nonsensical in Kestrel (no user-facing
reference types). Two types make this unrepresentable at compile time.
For drop elaboration: UseMode::Move is a kill, everything else isn't — clean
single match. For a future borrow checker: call-scoped borrows (ArgMode::Ref)
have trivially bounded lifetimes and can be handled differently from standalone
refs (Rvalue::Ref). Keeping them as separate representations preserves that
distinction.

ApplyPartial captures use UseMode, not ArgMode. Borrowed captures are
materialized as ref temps first. Closures are rare (~1-2 per function).

## Flat Place

**Old:** Recursive boxed enum: `Field { parent: Box<Place>, name: String }`.
Every projection allocated a Box. `root_local()` was a recursive walk.
Fields and variants were string-keyed.

**What broke:** String-keyed variants enabled display-name leak bugs (parens
in variant names) that the verifier had to detect. Overlap checking for
move/borrow analysis required recursive tree comparison. Clone was expensive
(heap tree walk). And string comparison at codegen time for enum switches.

**New:** `Place { base: PlaceBase, projections: SmallVec<[PlaceElem; 2]> }`.
FieldIdx(u16), VariantIdx(u16) resolved at lowering time.

**Why:** Flat places make overlap checking a slice prefix check — the core
operation for move checking and borrow checking. No heap allocation for ≤2
projections (the common case). Index-based fields/variants eliminate the
string bug class entirely. This is how Rust MIR's Place works.

**Reference:** Rust MIR `Place { local, projection }` with `ProjectionElem`.

## Type interning

**Old:** `MirTy` stored by value everywhere. Recursive `Box<MirTy>` children.
`clone()` walked trees. Three independent `substitute_type_params` functions
across ty.rs, drop_elaboration.rs, and verify.rs.

**What broke:** Type equality was recursive tree comparison. Every substitution
cloned and rebuilt trees. The three substitution implementations drifted
subtly (one used `&[(Entity, &MirTy)]`, another used `HashMap`).

**New:** `TyId(u32)` indexing into an append-only `TyArena`. One canonical
copy per structural type. One `substitute()` function.

**Why:** Equality is ID comparison. Substitution interns the result —
deduplication is automatic. The append-only arena allows `&TyArena` reads
alongside `&self` inserts (interior mutability). The triple-duplication of
substitution collapses into one function because there's one representation.

**Reference:** Rust `TyCtxt` with arena-allocated interned types.

## No Ref/RefMut in MirTy

**Old:** `MirTy::Ref(Box<MirTy>)` and `MirTy::RefMut(Box<MirTy>)` encoded
calling convention in the type. Drop elaboration checked
`!matches!(ty, Ref(_) | RefMut(_))` to determine if a param was consuming.

**What broke:** The type was doing two jobs: representing ABI (param is a
pointer) and encoding semantics (this is a borrow). Adding user-facing
references later would require disambiguating "calling convention Ref"
from "user-wrote &T."

**New:** `ParamConvention { Borrow, MutBorrow, Consuming }` on ParamDef.
The param's type is the unwrapped semantic type (String, not Pointer(String)).
Inside the body, borrow params have type Pointer(T).

**Why:** "Is this param consuming?" becomes `convention == Consuming` instead
of type unwrapping. If user-facing references are added later, `MirTy::Ref(TyId, Region)`
comes back as a semantic type with lifetime info — a clean slot, not an
overloaded one. This aligns with Swift SIL's approach where ownership convention
is on the value, not the type.

**Reference:** Swift SIL `@owned` / `@guaranteed` / `@borrowed` on values.

## Drop shims

**Old:** Drop elaboration was a 5-phase pipeline (2,200 lines):
1. Identify droppable locals (5-step scan)
2. Forward dataflow
3. Insert Deinit/DeinitIf markers
4. Expand markers into CFG + Call (up to 8 fixed-point iterations)
5. Inject field cascading in deinit bodies

`collect_struct_field_drops` was hardcoded to 3 levels of struct nesting.
The transitive detection stopped at 2 levels — the known cause of
String/Array/CowBox memory leaks (String → CowBox → RcBox chain missed).

**What broke:** The 8-iteration expansion loop was arbitrary and could
silently leave unexpanded Deinit nodes. The depth-limited field recursion
leaked ~8KB per HTTP request in Perch. Two separate drop systems existed
(one active in kestrel-mir, one written-but-unwired in kestrel-ownership)
with inverted flag conventions.

**New:** One `__drop$T` function per type that needs cleanup. Drop elaboration
inserts `Drop { place }` and `DropIf { place, flag }` statements that
reference the shim. These are final IR forms (not intermediate markers) —
codegen emits a call to `__drop$T` for `Drop` and a branch + call for `DropIf`.
The shim recursively drops fields to arbitrary depth. One drop system, one
flag convention.

**Why:** Shim synthesis is a fixed-point over the type graph — it naturally
handles arbitrary nesting depth. String → CowBox → RcBox → ... works
regardless of chain length. The expansion is structural recursion inside
the shim, not iterative CFG surgery. The two-system problem disappears.

## Unified droppable-local identification

**Old:** Three independent implementations:
1. `identify_droppable_locals` in drop_elaboration.rs (5-step, most complete)
2. `find_droppable_locals` in verify.rs (simpler, missed edge cases)
3. `MovePathSet::build` in move_check.rs (different criteria)

**What broke:** Semantic changes to "what counts as droppable" required
updating all three. The verifier's version missed the `call_result_targets`
exclusion, so it could disagree with drop elaboration.

**New:** Instead of a precomputed filter, a simple type query
`needs_drop(arena, module, ty) -> bool` determines at the point of use
whether a local's type needs cleanup. The init-state dataflow tracks ALL
locals (not just droppable ones), and the `needs_drop` check happens at
the insertion/verification point.

**Why:** The precomputed filter was a correctness surface — if it disagreed
with reality, drops were silently missed. Moving the check to the point
of action eliminates that surface. The dataflow tracking all locals is
also useful for move checking (the verifier needs "is this local
initialized?" for all locals, not just droppable ones). The cost is a
wider bitset in the dataflow — negligible for typical function sizes.

## Partial moves deferred

**Old risk:** Root-local dataflow treats a local as wholly Live, Dead, or
Maybe. If the IR allowed `move s.f` from a droppable aggregate before
projection-aware tracking exists, drop elaboration would either drop too much
or too little: killing all of `s` leaks siblings, while keeping `s` live risks
dropping the moved-out field.

**New v1 rule:** Partial moves are not supported in MIR-2 v1. The verifier
rejects `Use(Place(s.f), Move)`, `ArgMode::Move` on projected fields, and
`Return(s.f)` for owned droppable aggregates. Whole-local moves remain valid.
Projected assignment is allowed as an overwrite, with a field drop inserted
first when the old field value needs cleanup.

**Why:** This keeps the first drop elaboration implementation sound while
still preserving the flat Place representation needed for future move paths.
Projection-aware move paths can be added later as an extension of the
dataflow lattice, not as a prerequisite for MIR-2.

## Shared dataflow infrastructure

**Old:** RPO computation, predecessor maps, and worklist iteration were
independently implemented in drop_elaboration.rs, liveness.rs, verify.rs,
and move_check.rs. Four implementations of the same algorithm.

**New:** One `CfgInfo` structure and generic `forward_fixpoint`/`backward_fixpoint`
functions parameterized by a lattice and transfer function.

**Why:** The verifier should share infrastructure with the pass it verifies.
Subtle semantic differences between independent implementations go undetected.
One implementation means one set of bugs to find.

## Monomorphization as MIR pass / kestrel-codegen dissolution

**Old:** Three crates participated in monomorphization and layout:
- `kestrel-codegen`: layout cache (990 lines), name mangling (622 lines),
  target config (109 lines)
- `kestrel-codegen-cranelift`: monomorphization discovery (754 lines),
  type substitution (841 lines), witness resolution (725 lines)
- `kestrel-mir`: non-generic layout pass (143 lines)

Layout was computed on-demand during codegen. Witness resolution happened
twice (discovery + emission). Every codegen function threaded substitution
maps. Codegen was doing MIR-level work.

**New:** Monomorphization is a MIR pass in kestrel-mir-2 producing a
`MonoModule`. All types concrete, all witnesses resolved, all layouts
computed, all names mangled. The old `kestrel-codegen` crate dissolves:
- **Layout arithmetic** (StructLayout helpers, enum layout, padding) →
  kestrel-mir-2 shared infrastructure, used by both the non-generic layout
  pass and monomorphization Phase 4
- **Name mangling** → kestrel-mir-2, called during monomorphization Phase 3
- **TargetConfig** (pointer width) → passed as a parameter to layout
  and monomorphization, not stored on MirModule (IR is target-agnostic)

Codegen (kestrel-codegen-cranelift) becomes a pure emitter: it reads
`MonoFunction.name` for symbols, `MonoStruct.type_info.layout` for field
offsets, `Callee::Resolved(id)` for call targets. No computation.

**Why:** Codegen should emit IR for concrete types, not perform generic
resolution or layout computation. MonoModule enforced at the Rust type
level — codegen takes `&MonoModule`, cannot accidentally receive generic
MIR. Callee/Rvalue/Statement types are shared between generic and mono
MIR — the verifier enforces which variants are legal at each stage. No
duplicate IR types to maintain (see monomorphization.md "No separate
mono IR types").

**Reference:** Rust's monomorphization happens at the MIR level
(MonoItem collection), producing per-codegen-unit concrete MIR.

## Nullary ops as Immediate

**Old:** SizeOf, AlignOf, PtrNull, FloatConst were encoded as `Op1` with
a dummy argument. Worked but was semantically dishonest — these are constants,
not operations.

**New:** `ImmediateKind::SizeOf(TyId)`, `AlignOf(TyId)`, `NullPtr(TyId)`,
`FloatInfinity(FloatBits)`, `FloatNan(FloatBits)`.

**Why:** They're compile-time constants parameterized by type. After
monomorphization they can be folded to concrete integer values. No dummy
arguments, no arity mismatch between the Op and its Rvalue wrapper.

## MonoFuncId instead of Entity

**Old:** Function references in generic MIR use `Entity` (ECS identity).
Thunk synthesis used `Entity::from_raw(u32::MAX/2 + ...)` as synthetic IDs.
Drop shim synthesis used `u32::MAX/2 + 0x40000 + seed`. Collision risk.

**New:** MonoModule uses `MonoFuncId(u32)` — an index into the module's
function list. The monomorphizer assigns IDs during instantiation. Every
call target in MonoModule is a direct index.

**Why:** Closed, self-referencing module. No entity lookup, no collision
risk, no synthetic ID heuristics. Codegen indexes directly. The fragile
`u32::MAX/2` hack disappears.

## Drop flag convention

**Old:** Two systems with inverted conventions:
- kestrel-mir drop elaboration: `true` = skip (dead), `false` = needs drop (live)
- kestrel-ownership drop_elab: `true` = live (needs drop), `false` = dead (skip)

**What broke:** The inversion between systems was a bug waiting to happen
during the planned switchover.

**New:** One convention: `true` = live (needs drop), `false` = dead (skip).

**Why:** `true` = needs action is the natural reading. One system means
one convention.
