# Closure Kinds: Standard-Library Audit

**Date:** 2026-08-05  
**Design:** [Closure Semantics](closures.md)

This audit checks whether normal closures are the right default for Kestrel's
standard library. It inventories closure-taking declarations in
`lang/std/**/*.ks`, classifies them under the proposed closure kinds, and
records implementation consequences that do not belong in the language
semantics document.

## Inventory

The stdlib contains 133 declarations with closure-typed parameters:

- 124 explicitly public declarations;
- three protocol requirements;
- five private sort helpers;
- one fileprivate view initializer.

The user-facing surface is therefore 127 declarations.

Recommended classification of that surface:

| kind | declarations | share | rationale |
|---|---:|---:|---|
| normal | 98 | 77% | eager predicates, transforms, comparators, folds, and synchronous scoped bodies |
| `mutating` | 5 | 4% | explicitly side-effect-oriented eager operations |
| `escaping` | 24 | 19% | lazy adapters/views and their public constructors store the callback |
| `consuming` | 0 | 0% | no current stdlib callback needs to move a captured value out of its body |

Thirteen of the `escaping` declarations are public constructors for concrete
adapter/view types. Excluding those secondary construction APIs leaves 114
primary APIs: 98 normal (86%), five `mutating` (4%), and eleven `escaping`
(10%). Normal is therefore the clear default. The keyword count is noticeable
but concentrated in the lazy subsystem, where ownership is semantically
essential rather than stylistic.

## `escaping`: Callbacks Stored for Later Use

The following builder APIs return a value containing the callback and must
take an `escaping` closure:

- `Iterator.map`, `filter`, `filterMap`, `flatMap`, `scan`, `takeWhile`,
  `skipWhile`, `inspect`, and `intersperseWith`;
- `ArraySlice.split(where:)`;
- `Str.split(where:)`.

Their concrete adapter/view types contain thirteen closure fields, all of
which become `escaping`:

- nine iterator-adapter fields;
- two array split-view fields;
- two text split-view fields.

Their fourteen initializers—thirteen public and one fileprivate—take matching
`escaping` closure types. Existing `consuming` parameter conventions on those
initializers may remain; the kind is spelled independently in the parameter's
type. Aggregates containing the fields acquire the Cloneable retain/release
behavior defined by the closure design.

## `mutating`: Eager Write-Back Callbacks

The explicitly side-effect-oriented eager APIs should permit assignment to
captured frame variables:

- `Iterator.forEach` and `Iterator.tryForEach`;
- `Optional.inspect`;
- `Result.inspect` and `Result.inspectErr`.

Each takes a `mutating` closure through a `mutating` parameter. An inline
literal initializes the callee's mutable parameter storage directly. A normal
closure value uses the normal → `mutating` adapter: the adapter, not the
original `let` binding, supplies the mutable place. A pre-existing `mutating`
closure value must be held in `var`.

`tryForEach` currently forwards its action through a normal `tryFold` closure.
It should be implemented as a direct loop when closure kinds land; otherwise
its mutating action would force `tryFold` and the forwarding literal to become
`mutating` merely as an implementation artifact.

## Normal: Eager and Scoped Callbacks

All remaining callbacks stay normal. This includes:

- eager `map`/`filter` families;
- searches and predicates;
- sort comparators and key functions;
- fold/reduce callbacks;
- short-circuit thunks;
- `Pointer.with` and `Pointer.withMut`;
- `CowBox.modify` and `RcBox.modify`.

A `mutating` parameter inside the callback type, such as
`(mutating T) -> R`, controls access to the callback's argument and does not
by itself make the closure kind `mutating`.

This is an intentional policy boundary, not a claim that mutation could never
be useful inside `map` or `filter`. Those APIs remain read-only callbacks so
the common case stays callable from `let` and freely copyable. Callers needing
write-back can use `forEach`, an explicit accumulator (`fold`/`scan`), or a
shared object for state that must cross an escaping boundary.

If experience shows routine demand for write-back in the eager
transform/predicate family, the default should be reconsidered rather than
annotating most of the stdlib `mutating` piecemeal.

## Implementation Checklist

- Change thirteen stored closure fields to `escaping`.
- Change eleven lazy builder parameters and fourteen matching initializer
  parameters to `escaping` closure types.
- Change the five eager side-effect APIs to `mutating` closure types paired
  with `mutating` parameter conventions.
- Rewrite `Iterator.tryForEach` as a direct loop.
- Verify that closure-containing iterator types remain `not Copyable` when
  their source iterator is `not Copyable`, while eligible views become
  Cloneable through the aggregate fold.

## Test Matrix

- Inline literal passed to every `mutating` API, with captured write-back.
- Normal `let` closure adapted to every `mutating` API.
- Reusable `var` holding a `mutating` closure across repeated calls.
- Escaping literal passed through every lazy builder.
- Non-Copyable capture moved into each lazy builder and unavailable afterward.
- Copy/clone every eligible closure-containing adapter or view; verify shared
  state and exactly one last-release cleanup for acyclic environments.
- Reject a frame-view closure value passed into an `escaping` lazy-builder
  parameter.
- Verify `tryForEach` short-circuits without forcing `tryFold` to become
  `mutating`.
