# F39 — decisions

`lib/kestrel-lsp/src/handlers/completion.rs`'s `push_members_for_type` hand-rolled
member lookup: direct children of the receiver's nominal entity, then the children
of every `ExtensionsFor { target }`, then `ProtocolMembers` *only* if the receiver
entity itself was a protocol. It self-admitted the gap — `// Protocol conformances
aren't expanded here; M3 keeps it simple.`

Consequence, reproduced by unit test: **every protocol-extension default on a
concrete conformer was missing from `.`-completion.** `protocol Greeter` +
`extend Greeter { public func greet() }` + `struct P: Greeter` gave `["x","name"]`
from completion while `TypeMembers(P)` gave `["x(Direct)","name(Direct)",
"greet(ProtocolExtension{..})"]`. In the real stdlib that means every concrete
`Comparable` conformer lost `equal`/`notEqual`/`lessThan`/`lessThanOrEqual`/
`greaterThan`/`greaterThanOrEqual`/`isAtLeast`/`isAtMost`/`isBelow`, and every
`extend Iterator` block in `lang/std/iter/iterator.ks` (~16 combinators: `map`,
`filter`, `take`, `zip`, `sum`, `product`, `enumerate`, …) was invisible on
concrete iterator types.

Separately reproduced: **no visibility filter at all** — completing across modules
offered `private` and `fileprivate` members.

The fix replaces the three-branch walk with a nested-types pass plus a two-branch
dispatch onto the existing name-res queries, everything routed through a single
visibility gate. `push_members_for_type` gained a `context: Entity` parameter; the
caller passes the `body_entity` it already computes, matching the convention
`signature_help.rs` uses when it hands `context` to `TypeMembersByName`.

## 1. `TypeMembers` and `ProtocolMembers` stay separate — do not unify

**Decision:** dispatch on `NodeKind`. A `Protocol` receiver goes to
`ProtocolMembers { protocol, root }`; everything else goes to
`TypeMembers { type_entity, root }`.

The obvious simplification — "route everything through `TypeMembers`, it walks
conformances too" — is wrong. Both queries delegate to
`collect_members_transitive` (`lib/kestrel-name-res/src/traversal.rs`), and they
differ in exactly one argument: `include_parent_direct_children`.

* `TypeMembers` passes `false`. A conforming *type* never inherits a protocol's
  direct children — only protocol-*extension* defaults surface on it. That is
  correct: a bare requirement is not a member of the conformer, the conformer's
  own witness is.
* `ProtocolMembers` passes `true`. Protocol *inheritance* does pull in the
  parent's direct requirements.

So `TypeMembers(Comparable)` would omit `Equatable`'s direct requirement
`isEqual` — completing on a value whose static type is the protocol `Comparable`
would silently lose the inherited requirement. The two-branch dispatch is the
minimum that is correct for both receiver shapes.

No new query was needed: both queries are public and re-exported from
`kestrel-name-res` (`lib/kestrel-name-res/src/lib.rs`), and `MemberMap::iter()`
is `pub`.

## 2. The nested-types pass is a union, not a replacement — and it is direct-children-only

**Decision:** before dispatching, iterate `world.children_of(entity)` and push any
child whose `NodeKind` is `Struct` / `Enum` / `Protocol`.

Neither member query carries nested type *declarations*: `TypeMembers::is_member`
accepts Callable / Gettable / TypeAlias / EnumCase, and `ProtocolMembers::is_method`
accepts only Callable / Gettable. A naive swap to the queries would therefore have
silently dropped `Outer.Inner` from completion — a regression hidden behind a fix.
`member_completion_includes_nested_type` guards it.

**Direct children only.** The old code also walked extension children looking for
nested types; that was always a no-op, and the grammar proves it. In
`lib/kestrel-parser/src/extension/mod.rs`:

```rust
pub enum ExtensionBodyItem {
    Function(FunctionDeclarationData),
    Subscript(SubscriptDeclarationData),
    Initializer(InitializerDeclarationData),
    TypeAlias(TypeAliasDeclarationData),
    Field(FieldDeclarationData),
}
```

There is no `Struct` / `Enum` / `Protocol` arm, so a nested type in an extension
body is unparseable. Verified against the source, not assumed.

## 3. Visibility filtering is in scope *because* `TypeMembers` is deliberately unfiltered

**Decision:** add one choke point, `push_member_if_visible`, sitting between every
call site and the unchanged `push_member_entity`. It returns early unless
`ctx.query(IsVisibleFrom { target, context })`.

`TypeMembers` documents itself as "not name-filtered, not visibility-filtered, no
where-clause entailment — it's the union of every candidate," and `ProtocolMembers`
as "not visibility-filtered — witnesses dispatch private methods too." That is
correct for those queries: witness binding and conformance checking must see
private members. The filter conventionally lives one layer up, in
`TypeMembersByName` / `ProtocolMembersByName`, which call
`filter_members_by_name` → `IsVisibleFrom` (`lib/kestrel-name-res/src/helpers.rs`).

Completion consumes the *unfiltered* maps directly (it wants every name, not one
name), so it does not inherit that filter and must re-apply it. Routing every push
through one function — rather than sprinkling the check at the three call sites —
means a future fourth source of members cannot forget it.

Confirmed with teeth: stubbing the gate to a no-op makes
`member_completion_hides_cross_module_private_and_fileprivate` fail with
`got {"x", "hidden", "fp"}`.

None of the 10 pre-existing completion tests changed behavior — they are all
single-file / single-module with no explicit `private` / `fileprivate`.

## 4. `ResolvedTy::Named` remains the only handled receiver shape

Type-parameter, opaque, and `&T` receivers are a **separate** gap and were
deliberately not scoped in. Measured: `TypeMembers(T)` returns `[]` for both
`where T: Greeter` and `[T: Greeter]`, so completing on a generic receiver
produces nothing regardless of which query is used. Fixing that means resolving
the parameter's bounds to their protocols first — a different change with a
different blast radius.

## 5. `seen` stays keyed `name::kind`

Emission order out of `collect_members_transitive` is load-bearing and documented
as such: Direct → Extension → ProtocolExtension. `HashSet::insert` keeps the
first, so "a type's own member shadows an inherited default of the same name" is
what falls out for free — no explicit precedence logic needed.
`member_completion_direct_member_shadows_protocol_default` pins it by asserting
both that exactly one `greet` item is offered and that its `detail` is the
struct's direct signature.

## Open question (accepted trade-off, not an oversight)

`TypeMembers` returns protocol-extension members from **conditional / constrained
conformances unconditionally**. Where-clause entailment is explicitly the caller's
job — the query's own doc comment says so — and the machinery to discharge it,
`kestrel-analyze`'s `extension_clauses_entailed`, is private to that crate.

So completion will now **over-offer**: on a type whose conformance is constrained
by a where clause that isn't satisfied at the receiver's instantiation, members
from that constrained `extend` block appear anyway. This trades today's false
*negative* (the member is real and completion hid it — the F39 bug) for a false
*positive* (the member is offered and won't type-check if selected).

For an IDE that is the right side of the trade: an over-broad completion list
costs a red squiggle after selection, while a missing entry costs the user the
discovery entirely and is indistinguishable from the feature not existing. Same
reason `TypeMembers` itself is the union of every candidate.

Recorded as a **follow-up**, not an oversight. Closing it properly means either
exposing an entailment check from `kestrel-analyze` (or lifting it to
`kestrel-name-res`) and filtering `TypeMemberSource::ProtocolExtension { extension, .. }`
members against the receiver's concrete args, or de-emphasising rather than
dropping them (e.g. a lower `sort_text`, so they rank below unconditional members
but stay discoverable).

## Cheap optional follow-up

The `seen` key `name::kind` collapses **overloads** — two methods with the same
name and different argument labels yield one completion item. This is
**pre-existing**: the old code used the identical key, so the change neither
introduces nor worsens it, and it was left alone to keep the diff to the actual
bug. Widening the key to include the `Callable` parameter labels (the same string
`signature_detail` already builds for `detail`) would surface each overload as its
own item. Small, self-contained, and independent of everything above.
