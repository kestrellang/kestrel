# F3 — consolidating "is this a stored field?"

## Decisions (2026-08-08) — IMPLEMENTED

The two open questions below were resolved by the maintainer, and the answers are now in the code.
The rest of this document is the analysis that produced them; where it disagrees with this section,
this section wins.

1. **The classification is stored, not derived.** `FieldClass { backing, owner, is_static }` lives in
   `kestrel-ast-builder/src/components.rs` and is set once by `builders/field.rs` — the only code
   that inspects the accessor CST. Downstream sites call `FieldClass::is_stored_instance()` /
   `is_global_storage()` / `is_protocol_requirement()`. This is the "representation" angle, taken as
   the *primary* fix rather than the final cleanup stage, so no shared query is needed for the
   per-entity question.

2. **`var b: Int { get set }` on a concrete type is storage.** A bodyless accessor block declares how
   storage is accessed; only a *bodied* accessor (`{ get { … } }`, the `{ expr }` shorthand,
   `{ set { … } }`, `{ ref { … } }`) replaces storage with computation. In a `protocol` the same form
   remains a requirement, which `FieldOwner::Protocol` carries.

Consequence worth knowing: the `Computed` marker keeps its existing meaning — "declares an accessor
block", the *shape* question behind E413, E622 and doc rendering — and is no longer a storage
signal. Its doc comment says so.

Two follow-ups this exposed, neither pre-existing-fixed nor newly broken:

- `var b: Int { get }` on a concrete type is now read-only storage, which overlaps with `let b: Int`.
  No diagnostic distinguishes them.
- A bodyless accessor block in an `extend` block classifies as `FieldOwner::Extension`, which
  `is_stored_instance()` excludes — but there is *no* analyzer anywhere rejecting stored properties
  in extensions, so `extend S { var x: Int; }` remains silently accepted and unlowered. Pre-existing,
  out of scope here.

## TL;DR

The audit is right, and the four failures all **reproduce by running the compiler** (wrong-slot write on `S(a:1, c:3)`, E500 copy-fold false positive from a `static var`, E449 false positive from a `static var self-reference`, OSSA ICE on struct patterns with a computed property). Nothing here is speculative.

But the root fact is smaller than the site count suggests:

> On a `NodeKind::Field` entity, **`Callable ⇒ Computed`, never the converse.** `Computed` is set for *any* `PropertyAccessors` block (`lib/kestrel-ast-builder/src/builders/field.rs:66`); `Callable` only for a bodied getter, `{ expr }`, or a `ref` clause (`:117/:132/:141`). So `!Computed` is the **storage** test and `!Callable` is the **invocability** test, and every site that used `!Callable` to mean "stored" is wrong.

The fix is not one query. It is: **one classifier function that is the only place the predicate is written**, **one memoized roster query on top of it**, and **two fail-loud backstops** so the seventh divergence is a compile error instead of a wrong store. Land them in the order below; the first two stages are cheap and change nothing.

## Stages, cheapest and safest first

| # | Stage | Files | Changes what compiles? | Triage |
|---|---|---|---|---|
| 0 | MIR verifier: `InstKind::Struct` must supply every `FieldIdx` exactly once | 1 | No (all 4 producers already comply) | quick |
| 1 | Name-**checked** construct: bind labeled args by name, **ICE on unknown label** | 1 | No (turns a silent miswrite into a loud one) | quick |
| 2 | Add `kestrel-name-res/src/storage.rs` — classifier + rosters, no call sites migrated | 2 | No | none |
| 3 | Migrate the three already-correct sites + memberwise init to the API | 4 | No | quick |
| 4 | **Decision gate** (below) → new diagnostic for bodyless `{ get }`/`{ get set }` on a non-protocol parent | 2 | **Yes** | full |
| 5 | `struct_lower` uses the roster (layout/memberwise finally agree) | 1 | **Yes** | full |
| 6 | Copy fold: add the missing `Static` filter | 1 | **Yes** (E500 FP disappears) | full |
| 7 | Analyzers: struct_cycles, recursive_enum, protocol_field_conformance, initializer | 4 | **Yes** (E449 FP disappears) | full |
| 8 | Pattern rosters: pattern-matching + hir-lower | 2 | **Yes** (ICE → correct binding set) | full |
| 9 | Representation cleanup: builder writes the classification; markers stop being reverse-engineered | 3 | No | full |

Stages 0–3 are one small PR each and are safe to land on the shared branch immediately. Stage 4 is the gate — do not land 5 before it (see "ordering trap").

## The one decision only you can make

**What does `var b: Int { get set }` mean on a `struct`/`enum` — the bodyless accessor block on a non-protocol type?**

Today it is accepted, silently occupies a layout slot with no accessor and no way to read it, and is the direct cause of the index divergence. Three coherent answers:

- **(A) Reject it.** New E-code in `lib/kestrel-analyze/src/decl/field.rs`. A member with no storage and no body is meaningless outside a protocol. This makes the whole F3 class *unrepresentable* rather than merely *consistently handled*. My recommendation.
- **(B) Accept it as a computed property with a synthesized trivial accessor pair** (i.e. `Callable` gets set too). Then `!Callable` and `!Computed` reconverge and most sites become accidentally correct — but you have quietly added a language feature and the two predicates stay two.
- **(C) Accept it as storage** (make it stored, drop `Computed`). Cheapest, but it means `{ get set }` reads as documentation-only syntax, which will surprise everyone.

Everything downstream is mechanical once you pick. I have written the plan assuming **(A)**, and stage 4 exists specifically so the rejecting diagnostic lands *before* layout changes underneath it.

## The API (stage 2)

`lib/kestrel-name-res/src/storage.rs`. `kestrel-name-res` is below **every** consumer — I confirmed `kestrel-hir-lower/Cargo.toml:14` depends on it (the brief listed them as siblings), as do pattern-matching `:14`, semantics `:16`, mir-lower `:18`, type-infer `:15`, analyze `:19`. **Zero `Cargo.toml` edits.**

Two orthogonal axes, because the judges caught two real axes being folded away:

```rust
/// How a field is backed. `Accessors` covers ANY `PropertyAccessors` block,
/// bodied or not — this is the `!Computed` test, and it is THE storage test.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldBacking {
    /// No accessor block: real storage (inline, or a global — see FieldOwner).
    Inline,
    /// Declares accessors. `has_body` = the `Callable` marker = INVOCABILITY.
    /// Exposed only for witness dispatch / codegen. Never gate storage on it.
    Accessors { has_body: bool },
}

/// Where the field is declared. Load-bearing: `Static` is set only by the
/// `static` modifier (field.rs:243), so a MODULE-LEVEL `var g: Int = 0;` is
/// a Field with no `Static` — yet it is a GlobalRef, not inline storage.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldOwner { Nominal, Protocol, Module, Extension, Other }

pub struct FieldClass {
    pub backing: FieldBacking,
    pub is_static: bool,
    pub owner: FieldOwner,
}

/// THE classifier. The predicate exists here and nowhere else.
pub fn classify_field(ctx: &QueryContext<'_>, e: Entity) -> Option<FieldClass>;

// Derived helpers — each answers exactly one of the audit's questions.
pub fn is_stored_instance_field(ctx, e) -> bool;  // Inline && !static && owner==Nominal
pub fn is_global_storage(ctx, e) -> bool;         // Inline && (is_static || owner==Module)
pub fn is_protocol_requirement(ctx, e) -> bool;   // owner == Protocol
pub fn has_accessor_block(ctx, e) -> bool;        // Accessors{..}  (E413/E622/doc rendering)
pub fn is_invocable(ctx, e) -> bool;              // Accessors{has_body: true}
```

Roster query — **direct children only**, deliberately:

```rust
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StoredFields { pub type_entity: Entity }   // no `root` in the key
impl QueryFn for StoredFields { type Output = Arc<StoredFieldRoster>; }

pub struct StoredField { pub entity: Entity, pub name: String }
pub struct StoredFieldRoster {
    fields: Vec<StoredField>,          // position IS FieldIdx
    by_name: BTreeMap<String, u32>,    // BTreeMap for deterministic derived Hash
}
// iter / len / get / index_of(&str) / index_of_entity(Entity) / contains_name
```

**Incrementality — the part that matters.** The output carries entities, names, order. **No types, no spans, no visibility, no bodies.** Its derived `Hash` is the early-cutoff fingerprint for layout, copy semantics, drop shims, memberwise init, pattern arity and codegen; if a field *type* edit changed it, every keystroke would poison all of them. As specified, the fingerprint moves only when a field is added, removed, renamed, reordered, or gains/loses `Computed`/`Static`. That is strictly better than today, where `lower_struct` interleaves `resolve_type_annotation` into the walk and `NominalCopySemantics` re-walks children per call.

`StoredFields` issues **no sub-queries** — only `ctx.get` and `ctx.children_of` (one hierarchy dep on the parent, ~3 component probes per child). It is cycle-safe to call from inside `NominalCopySemantics` or `LowerTypeAnnotation`.

Sibling query `StoredGlobals { owner_entity }` for `Field && Inline && (static || module)` — a *separate* query, not a filter on a combined list, because a combined list is exactly how `semantics/lib.rs:744` folded statics into the copy fold in the first place.

## Fail-loud backstops (stages 0–1) — land these first

A shared query makes the right thing available; it does not make the wrong thing impossible. Two mechanical nets:

**Stage 0** — `lib/kestrel-mir/src/verify.rs`: every `InstKind::Struct` must supply each `FieldIdx` in `0..struct_field_count(ty)` exactly once. All four existing producers already emit full field lists, so this has no false positives today and catches every *future* producer.

**Stage 1** — `lib/kestrel-mir-lower/src/body/call/mod.rs:768-779`. `_struct_entity` is already a parameter and unused; `HirCallArg.label: Option<String>` already exists (`lib/kestrel-hir/src/body.rs:551-554`):

```rust
let idx = match arg.label.as_deref() {
    None      => FieldIdx::new(i),                       // synthesized/positional callers
    Some(l)   => roster.index_of(l).map(FieldIdx::from)
                 .unwrap_or_else(|| ice!("no field `{l}` on {struct_entity:?}")),
};
```

The `unwrap_or_else` is the whole point. **Do not** fall back to positional on an unknown label — that reintroduces F3 through a different door.

Stage 1 needs the roster, so if you want stage 0/1 before stage 2, do the lookup against `def.fields` via the existing `resolve_field_idx` (`context.rs:81-85`) and swap to the roster in stage 5.

## Ordering trap

Once stage 5 drops the phantom field from layout, `s.b` on a concrete `struct S { var b: Int { get set } }` resolves to no `FieldIdx` and no body — `resolve_field_idx` returns `None` and you get an ICE instead of a garbage read. **The rejecting diagnostic (stage 4) must land with or before stage 5.** The original plan deferred it to a separate later PR; that window is a regression.

## Site table (corrected)

| file:line | today | question | verdict |
|---|---|---|---|
| `mir-lower/items/struct_lower.rs:29-33` | `Field && !Callable && !Static` | layout roster | **wrong** — stage 5 |
| `type-infer/generate.rs:1341-1343` | `Field && !Computed && !Static` | memberwise init | correct — stage 3 (its comment falsely claims it "mirrors struct_lower") |
| `semantics/lib.rs:739-746` | `Field && !Computed` | copy fold | **wrong** (statics folded in) — stage 6 |
| `analyze/compilation/struct_cycles.rs:174-176` | `Field && !Callable` | E449 | **wrong** — stage 7 |
| `analyze/decl/recursive_enum.rs:183-186` | `Field && !Callable` | recursive enum | **wrong** — stage 7 |
| `analyze/decl/protocol_field_conformance.rs:66` | `Field` | conformance | stage 7 |
| `analyze/body/initializer.rs:151-171` | `!Callable` + **CST re-parse** | definite-init base | correct *by re-parsing the CST*; delete the workaround — stage 7 |
| `pattern-matching/constructor.rs:641-648` | `Field` | pattern arity | **wrong** (confirmed ICE) — stage 8 |
| `hir-lower/pat.rs:387-393` | `Field` | pattern name set | **wrong** — stage 8 |
| `mir-lower/items/witness_lower.rs:712-719` | `Field && Static && !Callable && !Computed` | stored static | correct; its comment ("getter `Callable` is on a child") is factually wrong — stage 3 |
| `mir-lower/items/static_value_type.rs:70` | checks both | stored static | correct — stage 3 |
| `mir-lower/body/mod.rs:2593-2601` `is_stored_global_def` | `Field && !Callable` | **module global** | needs `is_global_storage`, **not** a `Static` test — see below |
| `mir-lower/items/mod.rs:60-67, 91-97` | `Field && Callable` else `Field && Static` | item dispatch | **seventh site, missed by the audit** |
| `mir-lower/body/expr.rs:1699-1713` | bare `Field` + name match | subscript-setter routing | eighth site |
| `type-infer/captures.rs:173-191` | `!Callable && !Static && parent != Protocol` | capture place identity | closest to correct; keep its protocol guard |

Four factual corrections to the brief and to the leading proposal:

1. `lib/kestrel-mir-lower/src/ty.rs:761` is inside `#[cfg(test)] mod tests` (from `:567`). It is **not** a production layout authority. Production predicate (a) is only `struct_lower.rs:29`.
2. `struct_cycles.rs` and `recursive_enum.rs` are **`Field && !Callable`**, not bare `Field` — they're missing both filters, not just `Static`.
3. `is_stored_global_def` must **not** become a `Static`-requiring predicate. Its own doc comment explains that `Static` is too broad *and* it must cover module-level `var g: Int = 0;`, which carries no `Static`. That is why `FieldOwner::Module` exists in the API above. Migrating it to a naive `is_stored_global` would break every module-level global's `GlobalRef` path (`place.rs:129`, `expr.rs:1245`).
4. The protocol axis is **not** foldable into the storage axis. A bodyless, accessorless `protocol P { var x: Int; }` parses and produces a field with neither `Computed` nor `Static`; `captures.rs`'s parent-is-not-Protocol guard is load-bearing and must survive as `FieldOwner::Protocol`.

## Stage 9 — the representation fix

The runner-up proposal's real insight: the builder **already knows** the classification at `field.rs:66/117/132/141/245`. Everything downstream is reverse-engineering it from *absent* marker components, and absence is exactly what's easy to get wrong. Once stages 2–8 route all reads through `classify_field`, make the builder store `FieldClass` (or a `FieldKind` component) directly and have `classify_field` read it instead of inferring it. Then narrow `Computed`/`Callable`/`Static` visibility so a future site *can't* hand-roll the walk. This is behavior-preserving and is what actually prevents the seventh predicate — the query alone only makes the right thing convenient.

## What I would NOT do

- **Do not widen `TypeMember` / extend `TypeMembers`.** Wrong extent (it unions extension and protocol-extension members; layout needs direct children), wrong deps (it calls `ExtensionsFor`, documented in `lib/kestrel-hecs/issues.md#4` as an O(declarations) full-tree scan — a struct's layout would depend on every `extend` in the program), and widening its `Hash` re-executes every downstream memo. Sibling module, not a widened hot map.
- **Do not make the per-entity predicate a query.** Three `ctx.get` calls wrapped in a memo slot + deps vector + fingerprint is strictly worse than recomputing. The repo already states this test twice (`kestrel-type-infer/AGENTS.md` on `bind_arguments`; `kestrel-semantics/docs/architecture.md:37` on `protocol_allows_negative_conformance`).
- **Do not put field *types* in the roster output.** Tempting for sharing `resolve_type_annotation` work; it would make every field-type edit invalidate layout, copy semantics, drop shims, pattern arity and codegen. If profiling later shows double type-resolution dominating, add an index-aligned `StoredFieldTypes` query — but note that index-alignment between two queries is itself an F3-shaped fragility, so don't do it speculatively.
- **Do not "fix" the predicate by adding `!Computed` to `struct_lower` and stopping there.** That closes the headline miscompile and leaves an unchecked positional contract between five rosters. Stages 0–1 are what make the next divergence loud.
- **Do not standardize on `!Callable`.** It is the invocability test, and the bodyless-requirement / witness-dispatch distinction it draws is genuinely needed elsewhere. Conflating it with storage is the entire bug.
- **Do not edit existing testdata to make stages 6–8 pass.** If a fixture asserts the old E500 or E449, that assertion encoded a compiler bug — flag it and get an explicit call, per the repo rule.
- **Do not do this in one PR.** Shared branch: stage 2 is a *new file* plus one `lib.rs` export (lowest possible conflict surface); after that, one crate per PR, and scope every commit to explicit paths — never a bare `git commit`.

## Measured blast radius (better news than expected)

- All 16 testdata files containing `{ get }` / `{ get set }` declare them **inside `protocol` blocks**, which route through `protocol_lower::lower_protocol`, not `struct_lower` (`mir-lower/items/mod.rs:37/39`). Stage 5's `FieldIdx` shift touches **zero existing testdata**.
- `E449` appears in **zero** testdata files, so stage 7's false-positive fix has no annotations to renegotiate.
- Realistically ~18 files, ~230 new lines including tests, no `Cargo.toml` changes. `LowerCtx` already carries both `world: &World` and `query: QueryContext<'w>` (`mir-lower/src/context.rs:13-16`), so `classify_field(&ctx.query, child)` compiles at every mir-lower site with no plumbing.

## AGENTS.md (ask before adding)

Worth recording in `lib/kestrel-ast-builder/AGENTS.md`, next to the component definitions:

> On `NodeKind::Field`, `Computed ⊋ Callable`. `!Computed` is the **storage** test; `!Callable` is the **invocability** test. Never use `!Callable` to mean "stored". `Static` means the `static` modifier only — a module-level field is a global without it. All storage questions go through `kestrel_name_res::storage::classify_field`.

Repro programs (unmodified repo): `/private/tmp/claude-501/-Users-dino-Documents-Projects-kestrel/445b81e3-51d4-4bc8-9a2f-fa1dca82c3d1/scratchpad/f3audit/{repro,copy,cyc,pat}.ks`