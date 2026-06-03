# kestrel-mir

OSSA MIR data structures, monomorphizer, and passes. This file documents
known rules and invariants. It is not exhaustive — when you discover a new
rule, add it here.

## Verifier

`verify::verify_ossa` runs after lowering and after mono passes. It checks:
- Value uniqueness (no ValueId defined twice)
- Linear ownership (@owned consumed exactly once)
- Borrow scoping (EndBorrow before block exit)
- Block arg count, type, and ownership match
- Address init/uninit tracking (Uninit → StoreInit → Take)

It also checks that every operand has a definition (block param or
instruction result). A value used but never defined crashes codegen with
"ValueId not in value_map" — the verifier catches this earlier.

## Pass pipeline & stage dumping (`passes/mod.rs`)

`passes::Stage` is the **single source of truth** for the order and identity of
every observable point in the lowering → codegen pipeline. The variant
declaration order IS the pipeline order (and equals `Stage::ORDER`); the derived
`Ord` is what powers the `stop >= Stage::X` gating in `run_pipeline_until`. Ten
stages, in order:

- pre-mono (→ `MirModule`): `Raw` `DropFix` `Thunk` `DropShim` `CloneShim`
  `Layout` `Verify` — run by `run_pipeline_until` here.
- post-mono (→ `mono::MonoModule`): `Mono` `CopyProp` `Expand` — orchestrated by
  `Compiler::monomorphize_mir_until` in **kestrel-compiler** (the post-mono
  passes `eliminate_redundant_copies` + `mono::expand` + `verify_mono` live
  there, not here).

`run_pipeline` is now a thin wrapper = `run_pipeline_until(.., Stage::Verify)`;
keep it that way so codegen/`lower_to_mir` callers are unaffected.
`run_pipeline_until` gates each pass on `stop >= Stage::<pass>` and runs verify
ONLY at `Stage::Verify` — earlier stops return no errors so callers can dump a
not-yet-verified (malformed) module. `Stage::is_pre_mono()` is `self <= Verify`.

Inspect any stage with `kestrel dump mir -s <kebab-name>` (`--list-stages`;
`-s all` prints every stage). Default/`-s verify` aborts on verify error; every
other stage is best-effort (prints the module, verify errors → stderr warnings).
See [[mir3_dump_raw_debugging]].

**When you add or reorder a pass, update all four in lockstep:** the `Stage`
enum + `Stage::name`/`ORDER` here, the `run_pipeline_until` gate here (or
`monomorphize_mir_until` in kestrel-compiler for a post-mono pass), and the
`DumpStage` clap enum + `to_mir` mapping in `src/main.rs`. The kebab spelling
clap derives for `DumpStage` must match `Stage::name()`.

## Witness resolution

`mono/witness.rs::find_witness_with_method` matches witnesses by
`(protocol, self_type, method_key)`. For generic protocols like
`Convertible[From]`, multiple witnesses exist for the same
`(protocol, self_type)` pair (one per source type). The lookup filters
by `proto_type_args` using `witness_proto_args_match` — this compares the
witness's protocol type args against the expected args from the call site's
`method_type_args`. Without this filter, the first matching witness wins
regardless of which generic instantiation it represents.

`witness_proto_args_match` treats `TypeParam` entries as wildcards (they
come from `extend T: Proto[FreeParam]` where the free param has no
concrete value at witness-construction time).

## Per-type lookups in mono passes: key by `(Entity, type_args)`, never nominal alone

Copy/drop behavior is **per-instantiation**, not per-nominal. A conditionally-
Copyable generic (`Optional[T]` is `not Copyable` by default, Copyable via
`extend Optional[T]: Copyable where T: Copyable`) has `type_info.copy = Bitwise`
for `Optional[Int64]` but `None` for `Optional[File]` — both share one nominal
`Entity`. So any set/map a mono pass uses to decide copy/drop/clone treatment
must be keyed by `(Entity, Vec<TyId>)` (like `clone_lookup`/`shim_lookup` in
`mono/expand.rs`), and looked up with the value's actual `type_args`.

Keying by nominal `.source` collapses all instantiations: one move-only instance
poisons every instance. `expand_destroy_copy`'s `not_copyable` set hit exactly
this — it degraded a real `CopyValue` on `Optional[Int64]` into a move-alias, so
`let x = self.f; self.f = .None; x` (iterators, `take`/`replace`, `Heap.pop`)
returned the overwritten value. See [[expand_not_copyable_nominal_collapse]].
This is the MIR face of the same per-instantiation invariant the solver / MIR
ty_query / semantics enforce ([[per_instantiation_copy_semantics]]).

## Copy and destroy elaboration must stay symmetric (`mono/expand.rs`)

The `CopyValue` (copy) and `DestroyValue`/`DestroyAddr` (destroy) arms must agree
on **exactly which values carry resources**: anything the destroy side runs a
destructor on, the copy side must deep-clone (not bitwise-alias). Gate both on
the same predicate — `ty_needs_drop` — so they can't drift. If destroy drops a
member but copy aliases it, two owners free the same heap → **double-free**; if
copy clones but destroy skips, you **leak**. This applies per *operand-ownership*
case too: handle `@owned` and `@guaranteed` operands for **every** type kind the
destroy side handles, not just the convenient one.

This bites hardest on anonymous **tuples**, which have no nominal `Entity` and so
no `__drop$T`/`__clone$T` shim — their members are elaborated inline in both arms
(`emit_destroy_recursive` / `emit_clone_recursive`). `emit_clone_recursive`
projects members ByRef (`TupleExtract` on a `@guaranteed` operand), so to clone
an `@owned` tuple you must `BeginBorrow` it to a `@guaranteed` view first
(identity for an aggregate). Commit `22084a42` added tuple drop + clone but gated
the clone to `@guaranteed` operands only; an `@owned` tuple copy still aliased →
double-freed any heap member (crashed bootstrapped `flock` on
`Optional[(String, Int64)]`). Tests stayed green because they used string
*literals* — only **heap** strings corrupt malloc, so a correctness suite that
doesn't allocate won't catch this class. See [[tuple_drop_copy_elaboration_gap]].
