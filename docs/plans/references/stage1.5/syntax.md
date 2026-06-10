# Stage 1.5 — Syntax

> **Status 2026-06-10**: the place-accessor syntax below is IMPLEMENTED
> (items 1+5). One implementation nuance: `get`/`set` are RESERVED lexer
> tokens (pre-existing), while `ref` is genuinely CONTEXTUAL — the parser
> threads the source text through chumsky state and matches the
> identifier text in clause-head position only, so `ref` remains a legal
> identifier everywhere. A clause head must be followed by `{`; the one
> accepted grammar cost is a shorthand getter whose entire body is a
> trailing-closure call on a function literally named `ref` with a
> block-shaped closure body — parenthesize it. Named ref bindings and
> `&` patterns below remain FUTURE (item 2).

## Named ref bindings (decided direction)

`let r = &ring.peek();` — the visible `&` lands exactly on the construct
that can outlive its referent (`references-syntax.md` §2 Option C).
Prefix-`&` exists **only** in `let`/`var`-initializer position, so the
`a & b` bitwise-AND ambiguity is confined to that one restricted spot.
Bindings are block-local (no cross-merge — E-REF-15 still applies).

Fine semantics (PROPOSED, not ratified — the Swift `inout`-binding pitch
shape, which is the natural extension of transparent place):
- Bindings are `let`-only; **no `var r = &x`, no rebinding**.
- The binding *names the place*: `r = v` is **store-through** (legal only
  for `&mutating` bindings); there is no rebind spelling to confuse it
  with. (Rust distinguishes rebind from store via `*r = v`; Kestrel has
  no deref operator, so the binding form must not need one.)
- `let s = r` is a value context → **decays to a copy**; `let s = &r`
  re-borrows.
- Mutable twin: `let r = &mutating arr.mutableAt(index: i);` — `&mutating`
  mirrors the type syntax everywhere.

## Place accessors: `ref` / `mutating ref` — DECIDED 2026-06-10

Call-as-place for subscripts and computed properties is decided: two new
**accessor kinds** join `get`/`set` in the existing accessor-block grammar
(option A). Rejected alternatives: declared-ref-typed subscript pairs
matched by overload (leaks `&T` into signatures and the
`I.SeqOutput`-style protocol machinery; needs a novel pair-by-mutability
rule) and attribute-routed named methods (two spellings of one concept —
the §10.6 smell).

```kestrel
public subscript(index: Int64) -> T {
    ref {
        let count = self.len();
        if index < 0 or index >= count { fatalError("index out of bounds"); }
        self.ptr().offset(by: index).value
    }
    mutating ref {
        let count = self.len();
        if index < 0 or index >= count { fatalError("index out of bounds"); }
        self.makeUnique();
        self.ptr().offset(by: index).mutatingValue
    }
}

public var first: T {
    ref { self.ptr().value }
}
```

- **The declared type stays `T`.** Ref-ness is internal to the accessor:
  the body's expected type is `&T` (`mutating ref`: `&mutating T`) via
  the normal return-position implicit borrow; the signature clients,
  overload resolution, and generic code see is value-typed. No ref type
  ever appears in a subscript/property signature, so protocol
  associated-output machinery and E492 stay untouched.
- **Keyword is `ref`, not `deref`.** Accessors are named for what they
  provide from the author's seat (`get` provides a value, `set` consumes
  one, `ref` provides a reference). "Deref" names the use-site operation —
  an operator Kestrel deliberately doesn't have (transparent place). C#'s
  ref-returning indexers/properties are the closest precedent and spell
  it `ref`. Swift's `_read`/`_modify` imply yield/coroutine semantics ours
  don't have, and `modify` collides with the `storage.modify` idiom.
- In `mutating ref`, `mutating` modifies **the ref** (the accessor
  provides `&mutating T`), mirroring `&mutating`. Receiver conventions
  are defaults that come with the kind: `ref` → borrowing receiver,
  `mutating ref` → mutating receiver — right for COW containers, where
  `makeUnique()` genuinely mutates `self`. A future `nonmutating` prefix
  (Swift precedent: `nonmutating set`) is the escape hatch for non-owning
  view types; reserved, not built.
- Parser cost: `ref` is a contextual keyword inside accessor blocks only,
  exactly where `get`/`set` already are; the rule disambiguating `get {`
  from a shorthand-getter expression body extends unchanged, and
  `mutating` cannot start an expression at all.

### Declared `-> &T` on subscripts and properties

- **Subscripts: never.** E481 keeps rejecting declared-ref subscript
  types; the accessor form is the only spelling of a place subscript.
- **Computed properties: the stage-1 getter-only carve-out stays**,
  scoped as the low-level spelling for capability types. For
  `Pointer.value` / `.mutatingValue`, mutability is a property of the
  *handle*, not the receiver — writing through a pointer doesn't mutate
  the pointer (Swift's `UnsafeMutablePointer.pointee` is a
  `nonmutating set` for the same reason). Merging them into one
  accessor-form member would force a `var` receiver on every
  write-through. Guidance: ordinary API expresses places with accessor
  form (declared type `T`); declared-`&T` getter-only properties are for
  handles whose mutability lives in the type. Two forms with genuinely
  different semantics — not two spellings of one thing.

## `&` patterns — DECIDED 2026-06-10

Ref pattern bindings are spelled with the sigil, not a keyword:

```kestrel
match bucketRef {
    .Occupied(_, &v, _) => ...,          // v: &V — borrows the payload in place
    .Occupied(_, &mutating v, _) => ..., // v: &mutating V (mutable scrutinee place)
    _ => ...,
}
```

- **One cue everywhere a borrow gets a name**: `&` before a name means
  "this name borrows" — identical in `let r = &expr;` and in pattern
  binder position. A `ref` keyword would be a second spelling of the
  same cue.
- **`&mutating v` falls out of the type syntax**; the keyword
  alternatives (`ref mutating v` / `mutating ref v`) compose badly.
- **No Rust ambiguity is possible**: in Rust, `&p` patterns *peel* a
  reference (patterns are duals of expressions) while `ref v` binds by
  reference — opposite meanings. Kestrel's second-class refs cannot be
  structurally matched (scrutinees see through/decay), so the peeling
  meaning has no referent here and the binder meaning takes the symbol
  uncontested. Pattern grammar has no binary operators, so there is no
  bitwise-AND ambiguity either. Accepted cost: Rust users' priors read
  `&v` as the opposite; internal consistency wins.
- **Binder form only, not a structural operator**: `&name` /
  `&mutating name` are legal exactly where a binding pattern is; there
  is no `& .Some(x)` to "borrow a sub-pattern."
- Semantics prerequisite (the scrutinee-place rule) in `semantics.md`.
  Rides item 2's binding semantics — a borrowing pattern binding is a
  named ref binding scoped to the arm block.
