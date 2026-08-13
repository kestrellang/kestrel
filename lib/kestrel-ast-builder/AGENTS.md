# kestrel-ast-builder

Builds the hECS entity/component graph from the CST. This file documents known
rules and invariants. It is not exhaustive — when you discover a new rule, add
it here.

## This crate is the only place that reads accessor CST

Everything downstream sees components, not syntax. When you learn something
about a declaration while walking its CST, **store it as a component** rather
than leaving downstream code to reconstruct it. `FieldMutability` and
`FieldClass` both exist for this reason.

Corollary: never classify a declaration downstream by the *absence* of a marker
component. Absence is not a decision, and it silently acquires new meanings when
someone adds a declaration form that forgets to set the marker.

## Field storage: use `FieldClass`, never a missing marker

On a `NodeKind::Field` entity:

- **`FieldClass`** is the storage authority. Ask it via
  `is_stored_instance()` (occupies inline storage, i.e. gets a `FieldIdx`),
  `is_global_storage()` (backed by a `GlobalRef`), or
  `is_protocol_requirement()`.
- **`Computed`** means "declares an accessor block", bodyless or not. It is the
  accessor-*shape* question behind E413, E622 and doc rendering. It is **not** a
  storage signal.
- **`Callable`** means "has an invocable body/signature". On a field it is
  strictly narrower than `Computed` — `Callable ⇒ Computed`, never the converse
  — because `Computed` is set for any `PropertyAccessors` block while `Callable`
  is set only for a bodied one.
- **`Static`** means the `static` modifier *only*. A module-level `var g = 0;`
  is a global **without** it, which is why global-ness is
  `is_static || owner == Module` and not a `Static` test.

Using `!Callable` to mean "stored" is what caused audit finding F3: the layout
roster and the memberwise-init roster disagreed on bodyless `{ get set }`
fields, and struct construction maps argument position to `FieldIdx` with no
name check, so the mismatch was a wrong-slot write rather than an error. It also
produced a spurious E500, a spurious E449, and an OSSA ICE. See
`docs/design/f3-stored-field-consolidation.md`.

A bodyless accessor block on a concrete type is **storage** — it declares how
storage is accessed. Only a *bodied* accessor (`{ get { … } }`, the `{ expr }`
shorthand, `{ set { … } }`, `{ ref { … } }`) replaces storage with computation.
In a `protocol` the same form stays a requirement, carried by
`FieldOwner::Protocol`.

## An extension has no `TypeParams` for its LHS

`extend Box[T]` does not introduce `T` — it *binds* Box's own parameter entity.
So an extension entity carries `TypeParams` only for free parameters the
conformance RHS introduced (`extend Int64: ArrayIndex[T]`), and reading
`TypeParams` off an extension to answer "what generic parameters are in scope
here" silently returns nothing for every generic extension.

Ask `ExtensionLhsParams { extension, root }` (kestrel-name-res) instead. It is
the single answer, backed by the `ExtensionLhsParamNames` component this crate
writes once in `build_extension`. Do **not** reach for the target nominal's
`TypeParams` directly either — that over-approximates in the other direction and
leaks `T` into `extend Box[Concrete]` bodies, where the LHS bound nothing.

Both failure modes were real (audit finding F12): E439 never fired for a method
type parameter shadowing an extension LHS parameter, and `T` resolved inside
`extend Box[Concrete]`.

`collect_lhs_target_names` returns a source-ordered `Vec`, not a `HashSet`,
because its output is persisted in a component — see the ordering rule below.

## Component payloads must have deterministic order

Anything stored in a component and later iterated is part of the compiler's
observable output. Build these from ordered sources; a `HashSet`/`HashMap`
iteration order that leaks into a component reorders diagnostics, mangled names,
or emitted code between runs.

## Accessor children

A setter with a body lives on a spawned `NodeKind::Setter` **child**, not on the
field, and `ref`/`mutating ref` spawn `NodeKind::RefAccessor` children. Code
that enumerates a type's members must not assume every accessor-related entity
is the field itself.

## Predicates over `NodeKind`: exhaustive `match`, never `matches!`

`NodeKind::is_type_scope()` is written as a full `match` with no wildcard arm.
That is deliberate and worth copying for any new `NodeKind` predicate.

It replaced 7 copies of `matches!(k, Struct | Enum | Protocol | Extension)`
spread across this crate, HIR lowering, two analyzers and the LSP. Adding a kind
meant 7 lockstep edits; missing one reports *"cannot use 'self' in a static
method"* on correct code. With the wildcard gone, a new variant is a compile
error in exactly one place and the answer has to be given deliberately.

## `Name::ROOT`, never `"<root>"`

The root entity's name is a constant with an `is_root()` predicate. Compare with
those, never a literal — `kestrel-name-res`'s visibility check decides
top-level-ness by asking whether a parent is root, and it fails **open**: a miss
publishes every `internal` declaration with no diagnostic. `Name::ROOT` is
unspellable (`<` and `>` are not identifier characters) so it cannot collide.

## Operator spellings live on the op enums

`BinaryOp::symbol()`, `UnaryOp::symbol()`, and friends (in `kestrel-ast`) are the
only place an operator's source text is written. `operator_spellings_round_trip_through_the_lexer`
re-lexes each spelling and requires it to produce exactly the token the parser
maps back to that operator — so a spelling that is not real Kestrel syntax fails
the build. A second table in `kestrel-hir-lower` had written `&&`, `||` and `...`
for `and`, `or` and `..=`.

Relatedly: `token_to_binary_op` and friends return `Option`, and lowering now
treats `None` as `AstExpr::Error`. Do not reintroduce an `unwrap_or(BinaryOp::Add)`
— a fallback there compiles a different program than the one written.
