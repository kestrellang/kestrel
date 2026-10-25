# kestrel-name-res

Resolves names, type paths, and value paths against the hECS graph. This file
documents known rules and invariants. It is not exhaustive — when you discover a
new rule, add it here.

## Names are not identities — compare `Entity`

Two declarations routinely share a name. `struct Holder[T]` and a method
`func f[T]` inside it are *different* type parameters; so are an
`extend Box[T]` LHS parameter and a method's own `[T]`. Any check that decides
"is this the same declaration?" by comparing strings gets these wrong, and gets
them wrong **silently** — the inner declaration inherits the outer one's bounds,
associated types, or conformances with no diagnostic anywhere in the pipeline.

Resolve to an entity, then compare entities. Keep the name as a cheap prefilter
if the comparison is on a hot path, never as the decision.

Where-clause subjects follow this rule via `SubjectParam` / `subject_denotes`
(`resolve_type.rs`). The load-bearing detail: the subject must be resolved in
the scope of the entity that **bears** the clause, not in the asking context. A
clause on `Holder` resolves `T` to Holder's parameter, which is exactly what
makes a method's same-named `T` fail to match. This was audit finding F12.

## `resolve_type_param_assoc` walks two chains, and both matter

It searches the type parameter's own ancestors *and* the asking context's
ancestors. The second walk is not redundant: for `extend Box[T] where T: Mapper`,
`T` is Box's parameter, so the extension — which carries the where clause — is an
ancestor of the *context*, never of `T`. Removing either walk drops a legitimate
set of bounds.

## Extension type parameters come from `ExtensionLhsParams`

An extension entity's `TypeParams` component holds only free parameters
introduced by the conformance RHS (`extend Int64: ArrayIndex[T]`). LHS
parameters (`extend Box[T]`) bind the *target nominal's* entities and appear in
no `TypeParams` anywhere.

`ExtensionLhsParams { extension, root }` is THE answer to "which target
parameters does this extension's LHS bind". Neither shortcut is correct on its
own: reading the extension's `TypeParams` misses every generic extension,
and reading the target's `TypeParams` unfiltered leaks `T` into
`extend Box[Concrete]` bodies. Callers outside this crate (e.g. E439's
shadowing walk in kestrel-analyze) use the query too.

Matching is by **name** — `extend Box[U]` binds nothing, because Box declares
`T`. Making it positional is a deliberate behavior change, not a cleanup.

## Member lookup goes through `TypeMembers`, never a hand-rolled extension walk

"What members does this type have?" has exactly one answer: `TypeMembers` /
`TypeMembersByName` (`type_members.rs`). It walks direct children, then every
extension of the type, then every extension of every protocol the type
*transitively* conforms to, and tags each result with a `TypeMemberSource`
(`Direct` / `Extension(e)` / `ProtocolExtension { protocol, extension }`) so
callers can re-impose precedence without re-doing the walk.

Do not open-code `ExtensionsFor + VisibleChildrenByName`. Every hand-rolled
copy so far has lost something — and lost it silently:

- `find_in_extensions` returned only the first extension that matched, so
  splitting one `extend` block in two dropped every overload but the first.
  The set it produces becomes `HirExpr::OverloadSet` verbatim and inference
  never re-widens a single `Def`, so a drop here is unrecoverable downstream.
  Audit finding F10.
- The same function's protocol loop stopped at the first conforming protocol.
- LSP completion open-codes the walk and misses every protocol-extension
  member. Audit finding F39, still open.

`ExtensionsFor` on its own is still correct for questions that are *about the
extension* — which conformances it declares, which witnesses it supplies,
which target params its LHS binds. The rule is about member lookup by name.

### Precedence: label signature, not shadowing

When candidates from several sources collide, the rule (shipped in
`kestrel_type_infer::resolve_member`, mirrored in
`resolve_extension_static_method`) is: `Direct` and `Extension` candidates
compete equally, and a `ProtocolExtension` default joins the set only if its
**label signature** is not already taken. A type's own `tag()` therefore wins
over `extend SomeProtocol { static func tag() }` without the two becoming
ambiguous, while a protocol default with *different* labels stays reachable as
an overload. Suppressing protocol-extension candidates outright is the F10
mistake in the other direction.

## Lang items come from `@builtin` only

`ResolveBuiltin` reads `BuiltinIndex` and nothing else. Never add a
name-based shortcut back: a lookup by source name finds whatever the user
declared with that name (`module Int64` turned every integer literal into the
user's module and broke the stdlib — audit H2). The index keeps the first
annotation in declaration order and kestrel-analyze reports later ones (E400).
When an annotation sits on an alias whose *target* is the lang item (the
default literal types), say so in `Builtin::denotes_alias_target`.

## A type's lexical names are its member scope

`ScopeFor` on a struct/enum/protocol/extension lists the type's non-instance
members from **all** its parts (body + every extension), plus that part's own
type parameters. Instance members (fields, methods, subscripts, inits) are
never lexical bindings — only `self.x` reaches them (audit H3). Keep both
halves when touching `member_scope_children`: dropping the extension walk
makes a body and its extensions disagree about what `Inner` means; letting
instance members back in makes bare `count` resolve and fail in codegen.

## Cycle discipline

Walks over protocol inheritance or conformance carry a `visited` set —
`resolve_inherited_protocol_member` (`resolve_name.rs`) and the gatherers in
`conformances.rs`. A cyclic `protocol A: B` / `protocol B: A` otherwise
stack-overflows on already-invalid source. Add the guard when you add a walk.

There is exactly **one** inherited-associated-type walk, and it is
`resolve_inherited_protocol_member`. `resolve_type.rs` used to carry a copy
(`find_inherited_assoc_type`) that had drifted: no `visited` set, and an anchor
that climbed one extra ancestor per level. It stack-overflowed on a qualified
cycle (`protocol A: Test.B`). F37 deleted it and pointed
`search_protocols_for_assoc` at the shared function — do not reintroduce a
second copy. Details: `docs/fragility/F37/`.

Note that E459 does **not** protect these walks: `ProtocolCycleAnalyzer` is a
`CompilationCheck` that consumes name-res queries, so resolution runs underneath
the check that reports the cycle.

Similarly, prefer a narrow lookup over re-entering `ResolveName`/`ResolveTypePath`
when resolving something *inside* a declaration you are already resolving. That
is why the inherited walk resolves each conformance path from
`parent_of(protocol)` — the protocol's own declaring scope, recomputed per level,
never accumulated. `search_protocols_for_assoc` still climbs from the caller's
`scope` for its own where-clause bounds; that anchor is the same anti-pattern and
is a known follow-up (see `docs/fragility/F37/decisions.md`).

## `member_lookup_name` is the only answer to "what is this member called?"

`helpers::member_lookup_name` is `pub` because the conformance analyzers in
`kestrel-analyze` need it. There were three implementations, and they keyed
subscripts on **different components** — this one on the `Subscript` marker, the
analyzers on `NodeKind::Subscript`. They agreed only because the AST builder sets
both; nothing enforced it. `subscripts_carry_both_the_node_kind_and_the_marker`
(in `kestrel-ast-builder`) now does.

Note the shape of that failure: two representations of the same fact, read by
different code, kept in sync by nobody. `witness_lower.rs` documents a place
where the analogous `Callable`-vs-`Computed` split *did* drift. Prefer one
accessor over "check the marker" plus "check the NodeKind".

## A dotted path has exactly one walker

`resolve_type_path_chain` (`resolve_type.rs`) is the only implementation of
"resolve `A.B.C` segment by segment". It encodes the segment-ordering rules:
type-parameter associated types are tried before nested alias bounds, and a
bare `Self` short-circuits. `ResolveTypePath::execute` is a one-line delegate
that returns only `.resolution`.

If you need more than the final entity (every step, or whether the path was
`Self`-rooted), use the `TypePathChain` it returns. Do **not** add a second
query for it (that doubles the cache for one caller), and do not copy the
walk into a caller (that creates a second copy of the ordering rules to drift).
D7 commit 2 (`30b8f61c`) introduced the chain-returning function for exactly
this reason: `where C.Iter.Item: P` needed every step of the chain.
