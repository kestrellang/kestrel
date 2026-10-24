# Existentials (`any P`) Design

> **Status: design, not started.** Issue [#29](https://github.com/kestrellang/kestrel/issues/29),
> milestone 0.18. Base commit `80297742`. File and line anchors refer to that
> tree.

## Overview

An existential holds a value of *some* type conforming to a protocol, where
the concrete type is only known at runtime. `some P` (shipped in 0.16) hides
a type that the compiler still knows. `any P` erases it: one `Array[any Shape]`
can hold circles and squares together.

The design has two layers:

- **The raw existential**, written `rawany P` in this document. It is unsized:
  its size, alignment and methods come from a runtime function table. It has
  **no user spelling** and can never be held by value. It exists only inside
  a box.
- **The boxed existential**, `any P` and `any[B] P`. This is the only form
  users write. Both are pure syntax sugar for a box type applied to the raw
  existential:

| Written | Means |
|---|---|
| `any P` | `DefaultBox[rawany P]` |
| `any[B] P` | `B[rawany P]` |

`DefaultBox` is a builtin type alias, chosen the same way
`IntegerLiteralType` picks `Int64` for integer literals. Its default binding
is `RcBox`, so `any P` has reference semantics: copies share one payload. Because both forms are sugar, nothing after HIR lowering knows
that `any` was written. An existential is an ordinary generic instantiation
of a box over an unsized payload.

Today the compiler accepts a bare protocol as a value type, but nothing can
lower it (see [Current state](#current-state)). This feature replaces that
path.

## Syntax

```kestrel
protocol Shape {
    func area() -> Float64
    func name() -> String
}

// Default box (RcBox): copies share one payload.
let shapes: Array[any Shape] = [Circle(radius: 1.0), Square(side: 2.0)];
for s in shapes { print(s.area()) }

// An explicit box.
let owned: any[UniqueBox] Shape = Circle(radius: 3.0);   // one owner, move-only
let valued: any[CowBox] Shape = Square(side: 1.0);       // value semantics (needs Shape: Cloneable)

// Associated types pinned.
func sum(xs: any Iterator[Item = Int64]) -> Int64 { ... }

// Composition.
let x: any Shape and Printable = Circle(radius: 1.0);

// Downcast.
if let c = shapes(0) as? Circle { print(c.radius) }

// Type tests.
if shapes(0) is Circle { count += 1 }
match shapes(1) {
    c is Circle => print(c.radius),
    is Rect { width, .. } => print(width),
    _ => print("other"),
}
```

Grammar (`kestrel.ungram`, next to `TySome`):

```
TyAny = 'any' TypeArgumentList? Ty ('and' Ty)*
```

- `TypeArgumentList` holds exactly one argument: the box, as an
  **unapplied** generic type (`UniqueBox`, not `UniqueBox[T]`).
- The bounds after it are protocol references, optionally with
  associated-type bindings (`Iterator[Item = Int64]`), using the same syntax
  as `some P[Assoc = X]`.

A bare protocol in type position (`let s: Shape`) is an error. It is neither
an existential nor an opaque type.

## Semantics

### 1. The raw existential `rawany P`

- `rawany P` (and `rawany P and Q`) is a type the user can never write. It is
  produced only by desugaring `any`.
- **Unsized.** It does not satisfy the new `Sized` builtin (§2). It can only
  sit behind a pointer.
- **Copy class.** Never `Copyable`. It is `Cloneable` iff the protocol
  refines `Cloneable`. When that holds, every concrete payload is
  `Cloneable` too, so the table's clone slot is always filled.
- **Conforms to `P`.** `rawany P: P` is synthesized. Each requirement in the
  function table is witnessed by a dynamic call through the table (§5).

### 2. `Sized`

Boxes are user-definable, so "can this type parameter be unsized?" must be
part of the language, not a compiler-internal list.

- **New builtin.** `Sized` is an `implicit_conformance` builtin, like
  `Copyable` and `Static`.
  - Every concrete type is `Sized` except `rawany P` and structs whose
    **last stored field** is an unsized type parameter.
  - Those are "unsized-tail" structs. `RcBoxStorage[T]` and
    `UniqueBoxStorage[T]` are already shaped this way: a header, then
    `value: T`.
- **Implicit bound.** Every type parameter gets an implicit `T: Sized`. It is
  injected in the same place as the implicit `Copyable` bound
  (`inject_implicit_copyable_bounds`, `kestrel-type-infer/src/where_clauses.rs:296`).
- **Opt-out.** `where T: not Sized` lifts the bound, mirroring
  `T: not Copyable` (`TypeParamCopyRequirement`, `kestrel-semantics/src/lib.rs:269`).
  Inside such a declaration, `T` can only be used behind a pointer or as the
  unsized tail of a struct. Using it by value is an error: as a local,
  parameter, return type, or non-tail field.
- **Members needing `Sized`.** On a `not Sized` declaration, a member whose
  signature uses `T` by value is unavailable at unsized `T`. Examples are
  `UniqueBox.takeValue() -> T` and `CowBox.read() -> T`. This works exactly
  like a member in a `where T: Copyable` extension: the implicit
  `Sized` requirement is recorded on that member.
- **Associated types follow the same rule.** An associated type is
  implicitly `Sized` unless declared `type X: not Sized`.
  - `Indirection` declares `type Target: not Sized`, because
    `RcBox[rawany P]`'s `Target` is the raw existential.
  - A protocol requirement that takes or returns `Target` by value gets an
    implicit `where Target: Sized`, by the members rule above. That covers
    `ExistentialBox.init(consuming value: Target)`, for example.
  - Requirements that only use `&Target`, such as `pointeeRef` and
    `pointeeMutRef`, apply at any `Target`.
- **References to unsized types.** `&X` and `&mutating X` with an unsized
  `X` are two words (data, table), like `Pointer[X]`. They only appear
  inside the compiler: in the result of `pointeeRef()` on an existential
  box, and in thunks. Two existing rules handle them:
  - **Ref decay** (`&T` used where `T` is expected becomes a copy of `T`)
    never applies to an unsized `T`. If an expected type ever forces it,
    that is a type error, not a copy. User code can't trigger this;
    the rule guards against compiler bugs.
  - **The escape checker** treats a two-word reference exactly like any
    other reference. Provenance flows through it unchanged.

### 3. Boxes

A type `B` can appear in `any[B] P` iff:

1. **It conforms to `ExistentialBox`.** This new protocol in `std.memory`:
   ```kestrel
   public protocol ExistentialBox: Indirection {
       init(consuming value: Target)
   }
   ```
   It is the shape `SharedBox` already has (`sharedbox.ks:65`) minus the
   sharing requirements. `SharedBox` becomes a refinement:
   `SharedBox: ExistentialBox, Cloneable, MutableIndirection`. `init` is only
   ever called at a sized `Target` (§4).
2. **It has exactly one type parameter, declared `not Sized`.**
3. **It is unsizable.** Every stored field that mentions `T` is a
   `Pointer[X]`, where `X` is `T` or an unsized-tail struct ending in `T`, or
   is itself an unsizable box of `T`. This structural rule lets the compiler
   turn `B[T]` into `B[rawany P]` by widening those pointers (§4). An
   analyzer checks it once per box declaration, at the `not Sized` opt-out.

How the stdlib boxes fit:

| Box | Copying `any[B] P` | Mutating through it | Consuming through it | Payload may be move-only |
|---|---|---|---|---|
| `RcBox` (**default**) | Refcount bump (shares) | Yes, visible to every copy | No | Yes |
| `CowBox` | Refcount bump | Yes, forks first when shared | No | No: `CowBox[T]` requires `T: Cloneable`, so `any[CowBox] P` needs `P: Cloneable` |
| `UniqueBox` | Only if `UniqueBox[rawany P]` is `Cloneable`, i.e. `P: Cloneable` (deep clone) | Yes | Yes | Yes |

**Every row falls out of the box's own declaration.** Its `Cloneable` and
`MutableIndirection` conformances and their `where` clauses, evaluated at
`T = rawany P`, decide the behaviour. There are no existential-specific
copy rules ("depends on the box").

Required stdlib changes:

- **`UniqueBox`**: add `Indirection`, `MutableIndirection`,
  `ExistentialBox` and `ConsumingIndirection` conformances, plus
  `extend UniqueBox[T]: Cloneable where T: Cloneable`.
  - So `any[UniqueBox] P` is `Cloneable` exactly when `UniqueBox[rawany P]`
    is. That holds when `rawany P: Cloneable`, i.e. `P: Cloneable` (§1).
    There's no existential-specific rule.
  - At a sized `T`, `clone` allocates and clones the value as usual.
  - At an unsized payload, `clone` allocates using `sizeOfVal`/`alignOfVal`
    and copies through the table's `clone_into` entry.
- **`ConsumingIndirection`** (new, `std.core` next to `Indirection`). This is
  how a consuming requirement, or a consuming `as?`, gets the payload out of
  a box whose payload size is only known at runtime:
  ```kestrel
  public protocol ConsumingIndirection: MutableIndirection {
      /// Ends the box. `body` gets the payload's address and must move the
      /// payload out of it (or destroy it in place). The box then frees its
      /// storage without running the payload's deinit.
      consuming func consumePointee[R](_ body: (Pointer[Target]) -> R) -> R
  }
  ```
  - **Why a closure.** The box can't return the payload by value, because
    its type is unsized. Only the thunk, which knows the concrete `T`, can
    move it out. The closure keeps "move out, then free" in one call, so no
    caller can free the storage twice or forget to free it.
  - **Fits what exists.** `UniqueBox.takeValue()` becomes
    `consumePointee { p in p.move() }` at a sized `T`.
  - **Shared boxes don't conform.** `RcBox` and `CowBox` can't give up a
    payload that other copies still see, so consuming requirements need
    `any[UniqueBox] P`.
- **`RcBox`, `CowBox`, `UniqueBox`, `Pointer`**: add `T: not Sized`.
  - `RcBox`'s hand-computed `valueOffset = sizeof(Int64)` (`rcbox.ks:110`)
    becomes a layout computation that honours the payload's alignment.
  - `UniqueBox`'s equivalent changes the same way.
  - This is a latent bug for over-aligned payloads today, and a hard
    requirement once the alignment is only known at runtime.

### 4. Forming an existential

Conversion happens at **check leaves** in the bidirectional checker. It
replaces today's "protocol as value" rule (`solver.rs:2080`). With an
expected type `B[rawany P]`:

| Found | Rule | Effect |
|---|---|---|
| `T` where `T: P`, `T` sized | **Box** | Call `B[T].init(consuming:)`, then unsize |
| `B[T]` where `T: P` | **Unsize** | Widen `B`'s pointer fields to carry the `(B, T, P)` table. No allocation. |
| `B[rawany Q]` where `Q` refines `P`, or `Q` is a composition containing `P` | **Upcast** | Swap the table pointer for the parent table it carries (§5). No allocation. |
| `C[...]` with `C ≠ B` | **Error** | Boxes are never interconverted |

- **Recording.** The checker records the rule on the expression, like
  `PromotionInfo`, and MIR lowering emits it.
- **The box is part of the type.** `any[UniqueBox] P` and `any P` are
  unrelated types. To move a value into a different box, get the concrete
  value out first (with `as?`), or box the existential itself through
  `any P: P` (§6). The second option gives a box inside a box.

### 5. Function tables

One table per `(box B, concrete type T, protocol P)`. Associated-type
bindings are determined by `T`, so they don't need to be in the key.

- **Why the box is in the key.** Some entries depend on how the box is
  implemented, not just on `T` and `P`. A consuming requirement's thunk
  takes the box by value and frees it through that box's `consumePointee`.
  Keying every table by `B` keeps one rule (every table belongs to exactly
  one box) instead of mixing box-independent and box-specific entries.
- **What stays independent of `B`.** The slot *layout* depends only on `P`
  (`ExistentialLayout(P)`, below), so call sites never depend on `B` for
  slot indices. Type descriptors are per `T`, so `as?` and `is` compare the
  same identity whichever box holds the value.
- **Cost.** The same `(T, P)` stored in two different boxes gets two
  tables. Their non-consuming thunks are the same mono instances, so only
  the table data is duplicated.

**Layout.** The order is fixed by `P` alone, so call sites depend only on
`P`'s declaration:

```
header:   drop_in_place(ptr)
          size, align
          type_descriptor        // identity for `as?`
          clone_into(src, dst)   // present iff P: Cloneable
parents:  &table(B, T, P1) ... &table(B, T, Pn)   // in P's declared parent order
slots:    one thunk per dispatchable requirement, in declaration order
```

**Dispatchable requirements.** A requirement gets a slot unless it is one of
these:

- **static** (no receiver to dispatch on)
- **generic** (there is no finite set of instantiations to put in a table)
- **uses `Self` outside the receiver**, as a parameter or return type.
  Example: `Equatable.==(other: Self)`
- **mentions an associated type the existential leaves unbound**. Example:
  `next()` on a plain `any Iterator`

Calling one of these on an existential is a diagnostic that names the reason
(see [Error cases](#error-cases)).

**Protocol extension methods** are not requirements and never go in a
table. On an existential they run through `any P: P` (§6), monomorphized
with `Self = B[rawany P]`. So `Iterator`'s `map`/`filter` work on
`any Iterator[Item = Int64]`.

**Thunks.** Each slot is a synthesized function
`(self: Pointer[rawany P] erased to its data word, args...) -> ret`. It
re-types the data pointer as `T` and calls `T`'s witness. These thunks are
ordinary MIR functions and become mono roots, the same way escaping-closure
retain/release shims are built (`synthesize_env_shims`,
`kestrel-mir-lower/src/body/closure_box.rs:~467`). The thunk absorbs any
calling-convention difference, such as a by-value `Copyable` self.

**Composition.** `rawany P and Q` gets a table with an empty slot section
and parents `(B, T, P)` and `(B, T, Q)`, in canonical (sorted) order. Its methods
dispatch through those parent tables. Upcasting to `any P` is just loading a
parent pointer. Composition is therefore an anonymous protocol with parents
`P` and `Q`.

**Type identity.** `type_descriptor` is the address of a per-type static
symbol. Compilation is whole-program, so that address is unique per type.

#### Where tables live, and incremental compilation

- **When tables are made.** During monomorphization, on demand. A table is
  created the first time an unsize, box or upcast is reached for a given
  `(B, T, P)`. The key is interned like `InstantiationKey`
  (`kestrel-mir/src/mono/types.rs:11`). Compilation is whole-program: one
  `MirModule`, internal linkage everywhere
  (`codegen-llvm/src/context.rs:300`). So **no cross-module dedup, COMDAT or
  linkonce is needed**. One map in mono is the single source of truth.
- **New MIR.** These are the only new MIR items:
  - `VTableDef { key, entries }`, a static data item.
  - `Op::FuncAddr(instance)`, the address of a mono instance. No such
    instruction exists today: function pointers only appear as
    `ApplyPartial` words (`kestrel-mir/src/inst.rs:224`).
  - `Op::VTableAddr(key)`.
  - `Callee::Dynamic { table_word, slot }`.

  Both codegens emit `VTableDef` as a read-only global with function
  relocations.
- **Incremental.** Two separate queries:
  - `ExistentialLayout(P)` gives slot order, parents and dispatchability.
    It depends only on `P`'s requirement list, so editing a conformer never
    invalidates callers' slot indices.
  - `VTableFor(B, T, P)` depends on `T`'s witnesses for `P` and on `B`'s
    `ConsumingIndirection` witness. Today it is
    recomputed with the module. When MIR becomes incremental it is the unit
    to cache, and its inputs are already narrow.

### 6. `any P: P` (self-conformance)

**What it enables:**

- **Passing to generics.** `B[rawany P]` is accepted where `T: P` is
  required. A generic `func f[T: P](x: T)` and a `some P` parameter (generic
  sugar) both accept `any P` with `T = any P`. This is the only form of
  "opening" possible under monomorphization. A `T` bound to the dynamic type
  would need runtime instantiation, so it is not provided.
- **Protocol extension methods** on existentials work through it.

**How it is synthesized:**

- **Raw existential.** `rawany P: P` is synthesized (§1).
- **The box.** `B[rawany P]: P` is synthesized by forwarding through `B`'s
  `Indirection`: reads use `pointeeRef`, mutations use `pointeeMutRef`, and
  consuming requirements use `ConsumingIndirection`.

**When it holds.** The conformance holds **iff** both:

- every requirement of `P` is dispatchable (§5), and
- `B` grants the access level each requirement needs.

For example:

| Existential | Conforms to its protocol? | Why |
|---|---|---|
| `any Equatable` | No | `==` takes `Self` |
| `any Counter` with a `mutating` requirement | Yes | `RcBox` allows mutation (shared by every copy) |
| `any Closeable` with a `consuming func close()` | No | `RcBox` is not `ConsumingIndirection` |
| `any[UniqueBox] Closeable` | Yes | `UniqueBox` is `ConsumingIndirection` |

The diagnostic names the requirement that blocks it.

### 7. Member lookup

- **Today.** Lookup on an Indirection wrapper is wrapper-first: the peel
  only runs after the wrapper's members miss (`solver.rs:4255-4290`).
- **Existentials reverse it.** On a `B[rawany P]` receiver, lookup checks
  **the protocol's members first** (requirements, then extension methods),
  and the box's own members second.
  - This is an eager peel, like `&T` today, scoped to receivers whose
    `Target` is a raw existential.
  - So `RcBox.isUnique()` or `CowBox.read()` can never hide a protocol requirement
    of the same name.
  - The box's own API stays reachable when no protocol member has that
    name.
- **Operators and `for-in`** (`ctx.protocol_dispatch_members`) work through
  self-conformance (§6), not the peel.

### 8. Downcasting: `as?`

- **`x as? T`** on any existential checks the type descriptor in the
  table.
- **Result.** It returns `Optional[&T]`, borrowing the payload in place.
  This works the same in every box. With the existing ref-decay rule, `let c
  = x as? Circle` copies out when `Circle: Copyable`.
- **Consuming downcast.** `consuming x as? T` returns `Result[T, any[B] P]`.
  It is only allowed when `B: ConsumingIndirection`.
  - It checks the type descriptor first, through `pointeeRef`, without
    consuming anything.
  - On a hit: `.Ok(x.consumePointee { p in p.cast[T]().move() })`.
  - On a miss: `.Err(x)`, giving the existential back untouched.
- **Non-conforming target.** `as? T` where `T` cannot conform to `P` is a
  warning: the cast always fails.
- `as?` is new syntax. The `as` token exists (`kestrel-lexer/src/lib.rs:311`)
  but has no cast meaning yet.

### 8b. `is`: type-test operator and patterns

`is` is a **reserved keyword**. It has an expression form and a pattern
form. Both use the same runtime check as `as?`: compare the type descriptor
in the table with `T`'s.

#### The operator

```kestrel
if shape is Circle { count += 1 }
let circles = shapes.filter { it is Circle }
if shape is not Square { ... }
```

- **Result type.** It is typed exactly like a boolean literal.
  - `x is T` starts as an open slot that must conform to
    `ExpressibleByBoolLiteral`.
  - The context decides the type; otherwise it falls back to
    `@builtin(.DefaultBooleanLiteralType)` (`BooleanLiteralType = Bool`,
    `literals.ks:161`), through the same path literals use
    (`default_literal_var`, `solver.rs:5602`).
  - So `x is T` type-checks wherever `true` does and gets the same type.
    `let f: Flag = x is Circle` works for any `Flag: ExpressibleByBoolLiteral`.
- **Lowering.** The descriptor comparison yields a primitive `lang.i1`,
  which is passed to the chosen type's `init(boolLiteral:)`. That is free
  for `Bool`.
- **`x is not T`** is the negated form. `not` is already a keyword.
- **No narrowing.** `if x is Circle { ... }` does not change `x`'s type
  inside the branch, because Kestrel has no flow-sensitive typing. Use the
  pattern form `if let c is Circle = x` to get a name with the narrowed type.
- **Precedence.** Comparison level, non-associative:
  - `a is Circle == b` is a parse error;
  - `x is Circle and y is Square` is fine.
- **Left operand** must be an existential (`B[rawany P]`). A statically
  known type is an error (E152), since the answer is already known. A
  generic `T` is also rejected for now: answering per monomorphized
  instantiation would make generic code depend on `T`'s identity.
- **Right operand** must be a concrete, fully applied, sized type
  (`Box[Int64]`, not `Box`).
  - If it can't conform to every bound, that's the E149 "always fails"
    warning.
  - `is any Q` is out of scope, as with `as?`.
- **Consistency gap.** `Equatable.isEqual(to:)` returns `Bool` directly
  (`protocols.ks:36`), so `==` is fixed to `std.Bool` while `is` isn't.
  That belongs to the comparison protocols and is not changed here.

#### Patterns

| Pattern | Meaning |
|---|---|
| `is Circle` | Type test, no binding |
| `c is Circle` | Test, bind `c: &Circle` (a borrow, like `as?`; decays to a copy when `Copyable`) |
| `is Rect { width, .. }` | Test, then destructure as a struct pattern |
| `r is Rect { width, .. }` | Test, bind, destructure |
| `var c is Circle` | **Error E153**: an `is` binding borrows the payload |
| `Rect { width, .. }` on an existential | **Not** a type test, so the existing struct-pattern type mismatch. Type tests always say `is` |
| `x @ is Circle` | Legal; `x` binds the whole existential (`@` is unchanged) |

- **Where they work.** These patterns work in `match`, `if let`,
  `guard let` and `while let`.
- **`or`-patterns.** `c is Circle or c is Square` is rejected by the
  existing rule that a binding must have the same type in every
  alternative. `is Circle or is Square` is fine.
- **Exhaustiveness.** The set of payload types is open, so a match on an
  existential is never exhaustive without `_`.
  - `kestrel-pattern-matching` treats each `is T` as one constructor of an
    infinite domain, the way integer literals work.
  - A repeated `is T` gets the existing unreachable-arm warning.
- **Bindings always borrow.** Moving the payload out still goes through
  `consuming x as? T`.

### 9. Associated types

- **Binding.** `any Iterator[Item = Int64]` binds `Item` in the raw type
  (`rawany Iterator[Item = Int64]`).
  - Conversion requires `T.Item == Int64`.
  - Requirements mentioning `Item` become dispatchable.
  - `any Iterator[Item = Int64]: Iterator` holds, with `Item = Int64`.
- **Unbound.** Unbound associated types make the members that mention them
  non-dispatchable (§5). This also means self-conformance fails.
- **Constraint requirements.** An associated type with constraints
  (`type Item: Hashable`) stays satisfiable because the binding must
  satisfy them.

### 10. Interactions

- **Generics.** `T` may be instantiated with `any P`, since a box is
  `Sized`. `T` may never be `rawany P`: it can't be written, and `Sized`
  rejects it.
- **`some P`.** A function returning `some P` may return an `any P` (via
  self-conformance), but not the other way around.
- **Pattern matching.** `is` patterns (§8b) test and bind in `match`,
  `if let`, `guard let` and `while let`. `as?` composes with them.
- **References.** `&any P` borrows the box, an ordinary reference to a
  sized value. There is no user-visible `&rawany P`. The two-word
  `&rawany P` produced inside the compiler follows the rules in §2.
- **Escaping closures.** Unchanged. They keep their 4-word thick layout
  (`func_thick_words`). They are not re-expressed as existentials.
- **`static` / `Static`.** `rawany P` is `Static` iff every payload must
  be. Concretely, `B[rawany P]` follows `B`'s `Static` where-clauses, with
  `rawany P` treated as not `Static` unless `P: Static`.

## Current state

What the compiler does today on `80297742`:

- **Type checking.** The checker accepts a bare protocol `P` as a value type
  and accepts the `T → P` coercion (`solver.rs:2080-2091`). It treats `P`
  as `Copyable` (`solver.rs:2805`, `resolve.rs:2541`,
  `kestrel-semantics/src/lib.rs:612`) and as `Static`
  (`staticness.rs:368`).
- **MIR.** It lowers `P` as `MirTy::Named(P)` with no layout
  (`kestrel-mir-lower/src/ty.rs:130`). Witness calls on it can't be resolved
  at mono.
- **Net effect.** Programs using a bare protocol as a type type-check but
  can't run.
- **Affected tests.** Two diagnostics tests rely on the coercion, both with
  `stdlib: false`:
  - `declarations/extensions/extension_conformance_on_generic_type.ks:10`
  - `declarations/extensions/protocol_extension_satisfies_conformance_via_transitive_protocol.ks:10,23`

  Under this design both become invalid Kestrel. They are updated to
  `any P` in the same change that lands E144 (approved).

## Pipeline impact

| Stage | Change |
|-------|--------|
| **Lexer** | `any` and `is` keyword tokens (append to `kinds.txt`; regenerate) |
| **Parser / CST** | `TyAny` node; `types.rs` `at_type_start` + `base_ty` arm next to `some_type`; `as?` postfix and `is` / `is not` (comparison level, non-associative) in the expression grammar; `IsPat` node (`name? is Ty StructPatFields?`) in the pattern grammar |
| **Decls (ECS)** | `AstType::Any { box_ty, bounds, span }`. `@builtin(.DefaultBoxType)` alias slot plus `Sized`, `ExistentialBox` and `ConsumingIndirection` builtins in `kestrel-hir/src/builtin.rs`; `DefaultBoxType` added to `denotes_alias_target` |
| **Name res** | Resolve the box argument as an unapplied generic type |
| **HIR** | Desugar `any[B] P` → `HirTy::Struct{B, [HirTy::RawExistential{bounds, bindings}]}`, with `B = ResolveBuiltin(DefaultBoxType)` when omitted. A bare `HirTy::Protocol` in value position is an error. New `HirExpr::Cast { expr, ty, consuming }` for `as?`, `HirExpr::TypeTest { expr, ty, negated }` for `is`, and `HirPat::TypeTest { binding, ty, fields }` |
| **Type infer** | `TyKind::RawExistential`; `Sized` implicit bound plus the `not Sized` opt-out; box/unsize/upcast rules at check leaves, recorded per expression; synthesized `rawany P: P` and `B[rawany P]: P` in conformance queries; existential-first member peel; dispatchability errors; `is` typed as a boolean-literal slot; `is` patterns narrow their binding to `&T`. Remove the `Protocol`-is-`Copyable`/`Static` arms |
| **Pattern matching** | `TypeTest(T)` constructor over an open domain (exhaustiveness, unreachable arms) |
| **Analyzers** | Box validity (§3: one `not Sized` param, unsizable fields, `ExistentialBox`); unsized-by-value uses inside `not Sized` declarations; bare protocol type; non-conforming `as?` warning |
| **MIR** | `MirTy::RawExistential`; fat `lang.ptr` to an unsized pointee (2 words); unsized-tail layout with dynamic field offset; `sizeOfVal`/`alignOfVal`/`drop_in_place` on fat pointers via the table; `VTableDef`, `Op::FuncAddr`, `Op::VTableAddr`, `Callee::Dynamic`; thunk and table synthesis in mono |
| **Codegen** | Cranelift + LLVM: read-only table globals with function relocations, fat-pointer values, indirect calls |
| **Stdlib** | `ExistentialBox`, `ConsumingIndirection`; `SharedBox` refines `ExistentialBox`; `not Sized` on `Pointer`/`RcBox`/`CowBox`/`UniqueBox`; `UniqueBox` conformances; alignment-correct payload offsets; `@builtin(.DefaultBoxType) public type DefaultBox[T] = RcBox[T]` |

## Error cases

The codes are proposed, from the free ranges listed in
`lib/kestrel-analyze/AGENTS.md` (next free E1xx is E144; E402-E410 are
free). They need to be confirmed there before use.

| Code | Condition | Diagnostic |
|------|-----------|-----------|
| E144 | Bare protocol as a type | `protocol 'Shape' cannot be used as a type; write 'any Shape' for a boxed existential or 'some Shape' for an opaque type` |
| E145 | Non-dispatchable requirement called on an existential | `'zero' cannot be called on 'any Shape': it is a static requirement` (variants: `it is generic`, `it uses 'Self' as a parameter or return type`, `it mentions associated type 'Item', which 'any Iterator' leaves unbound`) |
| E146 | Existential passed where `T: P` is needed but self-conformance fails | `'any Equatable' does not conform to 'Equatable': requirement '==' cannot be called through an existential` |
| E147 | Conversion between boxes | `cannot convert 'any[UniqueBox] Shape' to 'any Shape': existentials in different boxes are different types` |
| E148 | Box lacks the access a call needs | `cannot call consuming method 'close' on 'any Closeable': 'RcBox' shares its payload, so it cannot give it up; use 'any[UniqueBox] Closeable'` (variant: `'CowBox' ...` / a box without `MutableIndirection` for mutating calls) |
| E149 | `as?` target doesn't conform (warning) | `cast from 'any Shape' to 'Int64' always fails: 'Int64' does not conform to 'Shape'` |
| E150 | Associated-type binding unknown | `'Iterator' has no associated type named 'Itm'` |
| E151 | Associated-type binding unsatisfied on conversion | `cannot convert 'Range[Int64]' to 'any Iterator[Item = String]': its 'Item' is 'Int64'` |
| E152 | `is` on a non-existential left operand | `'is' needs an existential on the left: 'Circle' is known at compile time` (variant: `'T' is a generic parameter`) |
| E153 | `var` binding in an `is` pattern | `an 'is' pattern binds a borrow of the payload, so it cannot be 'var'` |
| E402 | `B` in `any[B] P` is not a valid box | `'Array' cannot box an existential: its type parameter is not declared 'not Sized'` (variants: `it does not conform to 'ExistentialBox'`, `field 'items' stores 'T' by value`) |
| E403 | Unsized type parameter used by value | `'T' may be unsized here ('where T: not Sized'), so it can only be used behind a pointer or as the last stored field` |
| E404 | `any` box argument applied | `the box in 'any[...]' is written without type arguments: 'any[RcBox] Shape'` |

## Edge cases

- **Empty protocol** (`any Marker`): the table holds the header only. It
  is useful with `as?`.
- **Protocol with only extension methods**: there are no slots, and
  everything works through self-conformance.
- **Aliasing through the default box**: `let a: any Counter = ...; let b
  = a; b.increment()` changes what `a` sees. This is the intended
  consequence of `RcBox` as the default. `any[CowBox] P` gives value
  semantics when `P: Cloneable`.
- **Copyable payload in a `CowBox`**: fine. The clone slot is a bitwise
  copy.
- **Move-only payload in the default box** (`File` in `any Closeable`):
  - It boxes fine, because `RcBox` accepts non-Copyable payloads.
  - Reads and mutating calls work.
  - Consuming `close()` fails with E148. The fix is `any[UniqueBox] Closeable`.
- **Nested existential** (`any[RcBox] Shape` boxed from an `any Shape`):
  legal through self-conformance. The result is two allocations and double
  dispatch. That is intended, since §4 forbids silent re-boxing.
- **`any P` inside `Array`**: `Array[T]` requires `Sized`, and the box is
  sized. `Array[rawany P]` can't be written.
- **Recursive conformance**: `extend Circle: Shape` where a method returns
  `any Shape`. That is just a sized return type, with nothing special.
- **Same `(B, T, P)` reached from many sites**: one table, via the mono map.
- **Same `(T, P)` in two boxes** (`any Shape` and `any[UniqueBox] Shape`
  both holding a `Circle`): two tables that share their non-consuming
  thunks, with one type descriptor, so `is Circle` agrees across boxes.
- **Upcast through a diamond** (`P: A, Q: A`, from `any P and Q` to
  `any A`): composition parents are visited in canonical order, and the
  first path wins. All paths are identical by construction, since there is
  one table per `(B, T, A)`.
- **Generic payload** (`Box[Int64]: Shape` and `Box[String]: Shape`): two
  separate tables, keyed by the concrete instantiation.
- **`as?` to a protocol** (`x as? any Q`) is not in scope. Only concrete
  targets are supported. It needs conformance tables at runtime.
- **Over-aligned payloads**: payload offsets are computed from the table's
  `align`, which fixes the existing fixed-offset assumption in
  `RcBox`/`UniqueBox`.

## Open questions (resolved)

| # | Question | Resolution |
|---|---|---|
| 1 | What is `any P`? | Sugar for `DefaultBox[rawany P]`. The default box is a builtin alias, like the literal default types |
| 2 | Spelling of other boxes | `any[B] P` = `B[rawany P]`, also sugar. The raw existential has no user spelling; `&dyn P` is dropped |
| 3 | Box protocol | Lift `SharedBox`'s `init` into a new `ExistentialBox: Indirection`; `SharedBox` refines it |
| 4 | Box-to-box assignment | Not allowed. Move the value out (or wrap through `any P: P`) and re-box |
| 5 | Non-dispatchable requirements | Dropped from the table. Calling them is an error, and they block self-conformance |
| 6 | Associated types | In scope: `any P[Assoc = X]` |
| 7 | `any P: P` | Yes, synthesized, when every requirement is dispatchable and the box allows the access |
| 8 | Parent tables | Each table carries pointers to its parents' tables. Upcasts load one |
| 9, 10 | Mutation, move-only payloads | Decided by the box's own conformances and `where` clauses. No special rule |
| 11 | User-defined boxes | Yes: `Sized` builtin plus `not Sized` opt-out plus the structural unsizable rule |
| 12 | Table placement and incremental compilation | Mono-synthesized `VTableDef` keyed by `(B, T, P)`; layout query on `P` alone (§5) |
| 13 | Member lookup | Protocol members first, box members second |
| 14 | Bare `P` as a type | Error E144 |
| 15 | Composition, `as?`, opening | All in this feature. Opening is limited to self-conformance; there is no dynamic-type generic instantiation |
| 16 | Default box | `RcBox`: reference semantics, mutation always allowed |
| 17 | `ConsumingIndirection` | `consuming func consumePointee[R](_ body: (Pointer[Target]) -> R) -> R` (§3). A consuming `as?` returns `Result[T, any[B] P]` |
| 18 | `as?` syntax | Postfix operator |
| 19 | Existing bare-protocol tests | Approved to update to `any P` once the parser accepts `any` (see [Current state](#current-state)) |
| 20 | Type tests | `is` is a reserved keyword. The operator returns a boolean-literal-typed value (default `Bool`); `is not` negates; no narrowing; existential left operand only. Patterns: `is T`, `name is T`, `[name] is T { fields }`; `var` rejected; no struct-pattern sugar on existentials |
| 21 | Associated types and `Sized` | Implicitly `Sized` unless `type X: not Sized`; `Indirection.Target` is `not Sized`; by-value requirements get an implicit `where Target: Sized` (§2) |
| 22 | References to unsized types | Two words; ref decay never applies to an unsized type; the escape checker treats them like any reference (§2) |
| 23 | Cloning `any[UniqueBox] P` | Follows `UniqueBox[rawany P]: Cloneable`, from `extend UniqueBox[T]: Cloneable where T: Cloneable` (§3) |
| 24 | Table key | `(B, T, P)`: some entries depend on the box's implementation; the slot layout still depends on `P` alone (§5) |
