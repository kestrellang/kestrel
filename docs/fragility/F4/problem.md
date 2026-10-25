# F4 — the escaping-closure box `init` was picked by arity alone

> `medium` · `fragility` · `lib/kestrel-mir-lower/src/body/closure_box.rs`

## Diagnosis

`resolve_box_common` is the single helper both owning closure tiers go through:
`@builtin(.SharedBox)` (the `escaping` tier, `RcBox` today) and
`@builtin(.UniqueBox)` (the `consuming` tier). It resolved the box's
initializer with

```rust
let init = self.find_box_member(entity, NodeKind::Initializer, |c, _| c.params.len() == 1)?;
```

Two things are wrong with that line.

**1. The predicate is arity-only.** `find_box_member`'s `name` argument is dead
weight for initializers — `build_initializer` never attaches a `Name` component
to an init, so the `name` the predicate receives is always the empty string and
the only thing left to discriminate on is `params.len()`.

`lang/std/memory/rcbox.ks` has **two** one-parameter inits:

| line | signature | `label` | `is_consuming` |
| --- | --- | --- | --- |
| `:79` | `public init(consuming value: T)` | `None` | `true` |
| `:93` | `private init(inner inner: Pointer[RcBoxStorage[T]])` | `Some("inner")` | `false` |

Both satisfy `params.len() == 1`.

**2. `find_box_member` returned the FIRST hit.** It early-`return`ed inside the
scan loop, so of the two matches it silently took whichever the ECS enumerated
first — i.e. whichever was declared first in the source file. Everything worked
only because `:79` happens to precede `:93`.

The consequence of picking `:93` is not a diagnostic and not a crash in the
compiler. `init(inner:)` is the private adoption path used by `clone()`: it
stores its argument straight into `self.ptr` with no allocation and no refcount
write. `emit_box_env` hands it the *environment struct* positionally, so the
captured value's bytes land in the handle's pointer field. `emit_forget_handle`
then peels that field out as "the raw handle word" and packs it into word 1 of
the closure value. The first call through the closure dereferences a captured
`Int64` as a pointer.

## Reproduction

`lib/kestrel-test-suite/testdata/memory_model/closure_kinds/escaping/escaping_primitive_only_capture.ks`
is the minimal shape: an escaping closure capturing a bare `Int64`, created in a
frame that returns, then called.

The order dependency is made visible by swapping the two inits in
`rcbox.ks` so the private one is declared first — no other change:

```
=== OLD compiler (arity-only predicate + first-match-wins) with SWAPPED stdlib ===
RUN_EXIT=139        # SIGSEGV, no diagnostic

=== FIXED compiler (shape predicate) with SWAPPED stdlib ===
RUN_EXIT=0
```

With the stdlib in its shipped order both compilers exit `0`. The bug is
entirely latent behind declaration order.

## Why no dump stage shows it

The MIR dumps of the two compilers on the swapped stdlib are **byte-identical**
(`220595` lines each, `diff` empty). Both inits are named `init`, so the printer
renders both call sites as `call std.memory.RcBox.init[E](@mut_borrow %v4, %v1)`
— the only difference is the `Entity` inside `Callee::Direct`, which is not
printed.

The divergence first becomes visible after monomorphization, where the mangled
symbol carries the parameter label. Grepping the two `dump cranelift` outputs
for `RcBox.init` monomorphized at the closure environment type
(`<entity:Entity(2147482732)>`):

```
### OLD: RcBox inits monomorphized at the closure env type ###
_K0…_RcBox4_initEZm…RcBoxEI27_<entity:Entity(2147482732)>E L5_inner rN3_std6_memory7_PointerEI…RcBoxStorage…

### FIXED: RcBox inits monomorphized at the closure env type ###
_K0…_RcBox4_initEZm…RcBoxEI27_<entity:Entity(2147482732)>E 27_<entity:Entity(2147482732)>E …
_K0…_RcBox4_initEZm…RcBoxEI27_<entity:Entity(2147482732)>E L5_inner rN3_std6_memory7_PointerEI…RcBoxStorage…
```

The old compiler instantiates **only** the `L5_inner` (labelled `inner`,
pointer-taking) init at the environment type — proof that the boxing site chose
it. The fixed compiler instantiates the unlabelled payload-taking init there;
the `inner` one is still present because `clone()` legitimately reaches it.

## Fix

Three parts, in `closure_box.rs`:

1. **Match the requirement's shape, not its arity.** `is_box_init_shape` (a
   pure, ECS-free predicate) requires arity 1 **and** no label **and**
   `consuming` — the only legal shape of the `SharedBox` requirement
   `init(consuming value: Target)` (`lang/std/memory/sharedbox.ks:71`).
2. **Stop taking the first of N.** `find_box_member` now collects *all* matches
   and delegates to `exactly_one_box_member`, which panics `ICE: …` on both `0`
   and `≥2`. This applies to all four call sites (`init`, `sharedMutRef()`,
   `takeValue()`, `destroy()`), closing the same hole where two extensions could
   each contribute a same-named zero-parameter member.
3. **Defense in depth at the consumer.** `emit_box_env` builds its `CallArg`
   list positionally and never validates arity, so it now `debug_assert!`s that
   the resolved init still has the expected shape.

See `decisions.md` for why the predicate is a structural match rather than a
witness lookup, why the failure is a panic rather than a diagnostic, and for two
adjacent problems deliberately left open.
