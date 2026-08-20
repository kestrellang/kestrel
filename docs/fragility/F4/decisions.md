# F4 — decisions

## 1. Structural shape match (a), not protocol-witness lookup (b)

**Decision: (a).** `is_box_init_shape` requires
`params.len() == 1 && params[0].label.is_none() && params[0].is_consuming` —
the shape of `SharedBox`'s only initializer requirement,
`init(consuming value: Target)` (`lang/std/memory/sharedbox.ks:71`).

**Rejected: (b), `kestrel_name_res::find_protocol_witness_init`.** That is the
mechanism literal inits already use (`lib/kestrel-mir-lower/src/body/literal.rs:409-437`),
and it is the *principled* answer: ask the conformance which member satisfies
the requirement, rather than re-deriving the requirement's shape at the use
site. It is reachable for `SharedBox` — `RcBox` conforms via an extension in
`rcbox.ks`.

It does not work for the caller that actually needs it. `resolve_box_common` is
shared by both tiers, and the `consuming` tier's binding declares **no protocol
at all**:

```kestrel
@builtin(.UniqueBox)
public struct UniqueBox[T]: not Copyable where T: not Copyable {   // uniquebox.ks:66
```

`not Copyable` is a negative bound, not a conformance; there is no `UniqueBox`
protocol and therefore no witness table to consult. Adopting (b) would mean
`resolve_box_common` — one helper, one call site — running two different
resolution strategies depending on which builtin it was handed, which is exactly
the kind of split single-source-of-truth this audit exists to remove.

**Revisit condition.** If a `UniqueBox` protocol is ever introduced (the natural
companion to `SharedBox`, and the obvious place to state the
`takeValue`/`destroy` contract), switch both tiers to witness lookup and delete
`is_box_init_shape`. Until then the structural match is the only answer that is
uniform across the two tiers.

**Why (a) is safe rather than merely convenient.** `kestrel-analyze`'s
conformance-completeness checker already disambiguates this exact pair of inits
by per-parameter label
(`lib/kestrel-analyze/src/compilation/conformance_completeness.rs:1431-1445`,
`signatures_match`). If the label axis could not tell `init(consuming value:)`
from `init(inner inner:)`, `extend RcBox: SharedBox` would already be failing
E454/E458 in the shipped stdlib. The predicate here is strictly *tighter* than
what the analyzer enforces, so it can never accept something the analyzer
rejects.

**Known gap in that argument.** `signatures_match` compares arity and labels
only — it does **not** compare `is_consuming`. So a hypothetical `SharedBox`
implementation writing `init(value: T)` without `consuming` would pass
conformance checking and then fail `is_box_init_shape` with zero matches. That
failure is a loud `ICE:` panic naming the missing requirement, not silent
corruption, so it is an acceptable trade; the correct long-term fix is for
`signatures_match` to compare access modes too. Not done here — out of F4's
scope, and it touches a different crate.

## 2. Panic, not a diagnostic — and not `debug_assert!`

`find_box_member` used to return `Option<Entity>`, and every call site
`?`-propagated that `None` out of `resolve_box_binding`, where the caller reads
it as "no box available — fall back to a stack environment".

That fallback is real, but it belongs to exactly **one** `?` in
`resolve_box_common`: the `ResolveBuiltin` query at the top. A `// stdlib: false`
test genuinely has no `@builtin(.SharedBox)` type, and falling back to the
historical stack environment is the right answer there.

By the time `find_box_member` runs, three things have already succeeded: the
builtin resolved to an entity, the handle type interned, and
`unwrap_handle_to_pointer` peeled it to a raw machine pointer. The box type is
*proven present*. From that point a missing or duplicated requirement cannot
mean "no box" — it can only mean the binding is broken. Conflating the two is
what let a mis-selected member reach codegen.

So `exactly_one_box_member` panics on both `0` and `≥2` matches. A bare
`panic!("ICE: …")` is this file's established idiom for precisely this class of
"the binding must have supplied this" invariant — see
`binding.shared_mut_ref.expect("ICE: shared-box binding without \`sharedMutRef\`")`
and its `takeValue` / `destroy` siblings.

Deliberately **not** a `debug_assert!`: release is where the memory corruption
happens, and a check that vanishes in release does not guard the build that
matters. (`emit_box_env`'s arity re-check *is* a `debug_assert!` — that one is
redundant defense-in-depth over an invariant `exactly_one_box_member` already
established, not the guard itself.)

The fail-loud wrapper is applied to all four requirement lookups, not just
`init`. `sharedMutRef()`, `takeValue()` and `destroy()` are searched across the
type *and every extension of it*, so two extensions each declaring a same-named
zero-parameter member would have produced the same silent first-match-wins
selection. Only the tightened *predicate* is init-specific — the other three are
zero-parameter and have no label or access-mode axis to match on.

## 3. Open — recorded, not acted on

### 3a. `unwrap_handle_to_pointer` has the identical shape

`closure_box.rs:208-234`. It returns `Option<(Vec<TyId>, TyId)>` and `None`s out
on four distinct "the binding is malformed" conditions — a non-`Named`,
non-`Pointer` type in the chain; a missing `structs` entry; a wrapper struct
whose field count isn't 1; a wrapper chain deeper than the hard-coded bound of
4. Every one of those is a *broken shared box*, and every one is
`?`-propagated into the same "no box available, use a stack environment"
fallback as F4's bug — after the box type has already resolved.

This is the same resolved-but-then-silent-`None` shape and is a natural
candidate for the same `ICE:` treatment. It is a *different* failure mode
though: mis-shaped handles degrade to a stack environment rather than forging a
pointer, so the blast radius is smaller and the change would need its own
"is any legitimate binding non-conforming today?" check. **Left open on
purpose.**

### 3b. `rcbox.ks:37`'s doc example does not compile as written

```kestrel
/// let a = RcBox(value: [1, 2, 3]);
```

`RcBox`'s public init is `init(consuming value: T)` — a single-name parameter,
so the call site is positional. `sharedbox.ks:69-71` says so explicitly ("`value`
is a single-name parameter, so call sites are positional: `B(payload)`, never
`B(value: payload)`") and `rcbox.ks:195` writes the real form,
`RcBox(self.valuePtr().pointee)`.

Notable here because it is the same label axis F4 turns on: the doc comment
asserts a label that the requirement forbids. **Left open on purpose** — it is a
stdlib doc fix, and F4's brief is not to touch `lang/std/**`.
