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

## Cycle discipline

Walks over protocol inheritance or conformance carry a `visited` set —
`resolve_inherited_protocol_member` (`resolve_name.rs`) and the gatherers in
`conformances.rs`. A cyclic `protocol A: B` / `protocol B: A` otherwise
stack-overflows on already-invalid source. Add the guard when you add a walk.

Known gap: `find_inherited_assoc_type` (`resolve_type.rs`) is the same walk
*without* a guard — audit finding F37, still open.

Similarly, prefer a narrow lookup over re-entering `ResolveName`/`ResolveTypePath`
when resolving something *inside* a declaration you are already resolving.
`search_protocols_for_assoc` resolves from `parent_of(scope)` for this reason.
