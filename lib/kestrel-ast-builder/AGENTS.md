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

## Accessor children

A setter with a body lives on a spawned `NodeKind::Setter` **child**, not on the
field, and `ref`/`mutating ref` spawn `NodeKind::RefAccessor` children. Code
that enumerates a type's members must not assume every accessor-related entity
is the field itself.
