# Closures and Capture

A closure holds its environment the way its **kind** says it does, and the kind is spelled in the type. Two kinds are frame-bound and hold *views* of the enclosing frame; two *own* their captures and may outlive it. Everything else — capture mode, copy class, drop rules, escape rules — follows from the kind.

The full specification is [docs/design/closures.md](../design/closures.md); this page is the memory-model view of it.

## Basic Closure Syntax

```kestrel
let add = { (a: Int64, b: Int64) in a + b };
let result = add(1, 2);                       // 3

let double: (Int64) -> Int64 = { it * 2 };    // single param → `it` shorthand
numbers.map { it * 2 }                         // trailing closure
```

Closures are ordinary values: they can be bound to `let`/`var`, passed as arguments, stored in struct fields, and returned from functions — subject to the kind's rules below.

## The Four Kinds

```
fn_type ::= ['mutating' | 'consuming' | 'escaping'] '(' param_types ')' '->' type
```

| kind | captures | body may | callable | copy class | can leave the frame |
|---|---|---|---|---|---|
| **normal** — `(T) -> U` | views of the frame | read | many times, from `let` | Copyable | no |
| **mutating** — `mutating (T) -> U` | `&mutating` views | write back to the originals | many times; calls are exclusive (needs a `var`) | `not Copyable` | no |
| **consuming** — `consuming (T) -> U` | owned (moved/copied in) | move captures out | exactly once; the call consumes it | `not Copyable` | yes |
| **escaping** — `escaping (T) -> U` | owned snapshots, **shared** | read and mutate its own state | many times, from `let` | Cloneable (clone = retain) | yes |

Plain `(T) -> U` stays right for the vast majority of closures: `map`/`filter` callbacks, predicates, visitors. The keywords appear only when an API needs write-back (`mutating`), hand-off (`consuming`), or storage beyond the frame (`escaping`). There is no kind-on-literal spelling — the **expected type** selects the kind, and the literal is built for it.

## Capture Semantics

### View Kinds: Views, Not Snapshots

A normal or `mutating` closure's environment holds references into the frame — the same semantics as a named reference binding (`let r = &x;`). Reads see later writes, nothing is copied or moved:

```kestrel
var x = 10;
let f = { x };   // live view of x
x = 20;
f();             // 20
```

A `mutating` closure's views are `&mutating`, so its assignments write back (see below). A normal closure's captures are read-only: assigning to one — including a projection like `c.n = 5` — is **E603**, with a fix-it pointing at `mutating`.

```kestrel
var c = C(n: 1);
let bad: () -> Int64 = { c.n = 5; c.n };   // ERROR(E603): cannot assign to captured variable 'c'
```

### Owning Kinds: Snapshots

A `consuming` or `escaping` environment must own its contents, so each captured place is copied, cloned, or moved **at creation time** according to its copy class:

| captured place is | owning capture does | source afterwards |
|---|---|---|
| Copyable | bit-copy | untouched |
| Cloneable | `clone()` | untouched |
| non-Copyable, owned by the frame | **move** | dead — later use is **E500** |
| value carrying frame provenance (a view closure, a ref binding, a non-`Static` value) | rejected — **E624** | — |

```kestrel
var x = 10;
let g: escaping () -> Int64 = { x };   // snapshots x == 10
x = 20;
g();                                    // 10
```

The type tells you which semantics you have: **view kinds see later writes; owning kinds are snapshots.** The same body in a normal position would return 20.

### Captures Are Place-Based

The compiler captures the narrowest *place* the body actually uses, widening only when required:

```kestrel
{ self.data }          // captures the place `self.data`, not `self`
{ self.data.count }    // captures just `self.data.count`
{ self.method() }      // needs the receiver → widens to `self`
{ x + x.f }            // overlapping places merge → one capture of `x`
```

A closure inside a method on a `not Copyable` type can therefore use the receiver's fields freely without touching `self`. Owning capture of a projection such as `x.field` is a partial copy/clone/move; moving it leaves that projection dead and the aggregate partially moved, and ordinary partial-drop rules handle the rest.

*Limitation:* captures reached through a **nested** closure still collapse to the whole enclosing local rather than the narrowest place. This is a pinned v1 limitation, not a semantic rule.

### Non-Copyable Captures

A view kind does **not** move a non-Copyable capture — it aliases the original's storage, so the source stays live:

```kestrel
let r = Res(v: 42);      // Res: not Copyable
let f = { r.v };         // view — no move of `r`
f();                     // 42
f();                     // 42 again — and `r` is still live, no use-after-move
```

An owning kind *does* move it (E500 on later use). Inside a multi-call body — normal, `mutating`, or `escaping` — a capture cannot be moved *out*: that is **E506**. Only a `consuming` body, which runs at most once, may move its captures out.

### The Freeze Rule (E507)

A view must not dangle. While a live normal or `mutating` closure value may carry a view of a place, that place is **frozen against destruction**: it cannot be moved, passed to a `consuming` parameter, or destroyed with `deinit x;`. Plain reassignment stays legal — that is a write through a live view, which references already permit.

```kestrel
func sink(consuming r: Res) { }

let r = Res(v: 1);
let f = { r.v };   // views `r.v`
sink(r);           // ERROR(E507): cannot consume 'r.v' while a closure capturing it is live
f();
```

The freeze is **place-granular** (capturing `self.data` does not freeze all of `self`) and **lexical**, not use-liveness-based. Capture provenance propagates through closure copies, aggregate construction, assignments, parameters, and control-flow joins; every binding that may carry the view extends the freeze to the end of its own lexical extent.

The rule also has a scope-depth half: storing a view-carrying value into a binding that **outlives** the captured place is E507 even when nothing is ever moved.

```kestrel
var g: () -> Int64 = { () in 0 };   // capture-free literal: carries no view
if cond {
    let r = Res(v: 9);
    g = { r.v };                    // ERROR(E507): a closure viewing 'r.v' cannot outlive 'r.v'
}                                   // `r` dies here; `g` would dangle
g()
```

E507 is the closure analogue of E498 ("cannot consume a value while a reference into it is live"). Owning kinds freeze nothing: their captures are theirs.

## `mutating`: Write-Back

`mutating` captures `&mutating` views, so assignments hit the enclosing variables. Calling one is an **exclusive** use, so it must be held in a `var` (or a `mutating` parameter) — calling a `let`-held `mutating` closure is **E203**. The value is `not Copyable`, which keeps mutation sound with no aliasing analysis, and like a `mutating` method it cannot leave its frame.

```kestrel
var total: Int64 = 0;
var count: Int64 = 0;
var bump: mutating (Int64) -> () = { total = total + it; count = count + 1; };
bump(5);
bump(7);
// total == 12, count == 2 — the writes went back
```

In the stdlib, `Iterator.forEach` / `tryForEach` and `Result` / `Optional`'s `inspect` / `inspectErr` take `mutating` closures, so the accumulator idiom just works:

```kestrel
var total: Int64 = 0;
[1, 2, 3, 4].iter().forEach({ (x) in total = total + x });
// total == 10
```

There is no exclusivity rule: two `mutating` closures over the same place are permitted (Kestrel is single-threaded and deterministic).

## `escaping`: A Shared, Stateful Object

An `escaping` closure owns snapshots of its captures in a **shared heap environment** (a `SharedBox` — by default the refcounting `RcBox`; see [shared-box.md](../design/shared-box.md)). It is **Cloneable, not bitwise-Copyable**: duplicating a handle retains the environment rather than duplicating it, so copies **share state**. This is the deliberate exception to Kestrel's value semantics, and the keyword marks it.

```kestrel
func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }   // snapshots count; mutates its own copy
}

let next = makeCounter(10);
next();             // 11
let alias = next;   // retain — shares the environment
alias();            // 12
next();             // 13 — one counter, two handles
```

Because duplication goes through the ordinary Cloneable machinery, the aggregate copy fold does the right thing for free: a struct holding an `escaping` field becomes Cloneable and shares (never bit-copies) the handle.

```kestrel
struct Button {
    let onClick: escaping () -> ()   // storable long-term; Button is Cloneable
}
```

Captures in an acyclic environment are dropped exactly once, at the last release — deterministic cleanup, so a captured resource's `deinit` runs at a predictable point. **Caveat:** refcounting does not collect strong cycles. A cycle of escaping environments leaks, and its captures are never deinitialized; break it explicitly until weak references exist.

## `consuming`: One-Shot Hand-Off

A `consuming` closure owns its captures in a unique (unshared) environment. Calling it **consumes** it, so a second call is an ordinary use-after-move (**E500**). Because it runs at most once, its body is the one place allowed to move captures *out* — E506 is lifted there. It may leave the frame (it owns everything it holds), and it is `not Copyable`, so handing it around is a move.

```kestrel
func transfer(consuming r: Res) { }
func onDone(consuming f: consuming () -> ()) { f(); }

let file = Res(id: 3);
onDone({ () in transfer(file) });   // `file` moves in, then out — deinit runs exactly once
```

A `consuming` closure that is never called still drops its environment, dropping every capture exactly once.

## Passing: What Fits Where

The expected type declares both how the callee may call the closure and what ownership representation it receives. A literal is built directly for the expected kind. For existing closure **values**:

| have ↓ \ expected → | normal | `mutating` | `consuming` | `escaping` |
|---|---|---|---|---|
| normal | ✓ | ✓ | ✗ frame view is not owned | ✗ frame-bound |
| `mutating` | ✗ | ✓ | ✗ frame view is not owned | ✗ frame-bound |
| `consuming` | ✗ | ✗ | ✓ | ✗ one-shot |
| `escaping` | ✓ | ✗ shared, not exclusive | ✓ | ✓ |

A rejected cell is **E624**, with a note naming the property the source cannot supply. A conversion never recompiles the body or grants it new capture powers: normal → `mutating` is an exclusive-call adapter over the same frame views, and `escaping` → normal produces a non-owning *view* of the shared environment (frame-bound, rooted at the original handle).

The kind also dictates the parameter convention needed to call it: a `mutating`-kind parameter must be declared `mutating`, and a `consuming`-kind parameter `consuming`. A mismatch is **E625**.

```kestrel
func run(action: mutating () -> ()) { }
// error[E625]: a 'mutating' closure parameter must have the 'mutating' access mode

func run(mutating action: mutating () -> ()) { }   // OK
```

## Capture-Free Closures

A closure with no captures (or a named function used as a value) is a bare function pointer: it has no environment, never allocates, satisfies **every** kind, and escapes freely.

```kestrel
func constant() -> () -> Int64 {
    { 42 }                     // no captures: freely returnable
}

let cb: escaping () -> Int64 = { 7 };   // no allocation; null environment
```

## Parameter Conventions

Closure *parameters* carry access modes like function parameters. The convention may be written on the literal or **inferred from the expected type** — including a `let` binding's annotation:

```kestrel
struct Counter { var n: Int64 }

func bump(mutating c: Counter, with f: (mutating Counter) -> ()) { f(c); }

// literal omits `mutating`; inferred from the annotation
let f: (mutating Counter) -> () = { (x) in x.n = x.n + 10; };
var c = Counter(n: 0);
bump(c, with: f);      // c.n == 10
```

Conventions are checked contravariantly: a `mutating`-param closure cannot be passed where a plain (borrowing) closure is expected. This is the convention on a closure's *parameters*, and is independent of the closure's own **kind**.

## Escape Rule: View Kinds Stay in the Frame

A view-kind environment lives in the frame that created it, so the provenance escape checker (the same **E494** machinery used for references — see [diagnostics.md](diagnostics.md)) rejects any route out of that frame. The check is provenance-based, not syntactic — laundering through a binding or a struct field is still caught:

```kestrel
func makeAdder(n: Int64) -> (Int64) -> Int64 {
    { it + n }   // ERROR(E494): captures local `n`, which does not outlive the call
}

struct Holder { var f: () -> Int64 }

func make() -> Holder {
    let n: Int64 = 41;
    Holder(f: { () in n + 1 })   // ERROR(E494) — root tracked through the field
}
```

E494 on a closure carries a fix-it note suggesting an owning kind. Writing `escaping (Int64) -> Int64` (shared, multi-call) or `consuming (Int64) -> Int64` (unique, one-shot) in the return type rebuilds the literal with owned captures and makes it returnable:

```kestrel
func makeAdder(n: Int64) -> escaping (Int64) -> Int64 {
    { it + n }   // OK: owned snapshot of `n`, heap environment
}
```

A struct holding a view-kind closure field becomes frame-bound the same way; an `escaping` field keeps the struct storable long-term.

Capturing a **reference binding** (`let r = &x;`) in a view kind is legal — the view is frame-bound, so it cannot outlive the borrow. (This is why E212 is **retired**.) An owning kind still rejects it: there is nothing durable to own (E624).

## `return` Inside a Closure

`return` in a closure body returns **from the closure**, not from the enclosing function.

## Loop Variables

A view-kind closure over a loop iteration binding cannot outlive that iteration's lexical scope (E507). An `escaping` or `consuming` expected type snapshots each iteration's value, which is what makes accumulating closures work:

```kestrel
var fns = Array[escaping () -> Int64]();
for i in 1..=3 {
    fns.append({ i * 10 });   // element type supplies the escaping kind
}
(fns(0))();   // 10 — still valid after the loop scope died
```

---

## Design Notes

- **Copy and drop follow the kind.** View environments own nothing, so they need no drop; the frame's locals drop as usual. A `consuming` environment is dropped by its sole owner. An `escaping` environment's captures drop exactly once at the last release. A verify-time assertion enforces that no bitwise-copyable closure representation ever owns a droppable environment.
- **Exclusivity**: `mutating` closures are `not Copyable` and their calls need a mutable place; there is no separate uniqueness rule for `&mutating` captures.
- **Nested closures** capture transitively: an inner closure using an outer function's local forces the outer closure to capture it too. A frame-view inner closure stays rooted in the frame that created it and cannot escape merely because the outer closure owns its own environment.
- **Recursive closures** (`let f = { ... f(...) ... }`) are not expressible — the binding is not in scope inside its own initializer. Use a named function.

## Diagnostics

| code | when |
|---|---|
| [E203](../error-codes.md#e200e211--mutability-access-modes--assignment) | calling a `mutating`-kind closure held in a `let` (calls are exclusive) |
| [E212](../error-codes.md#e200e211--mutability-access-modes--assignment) | *retired* — view capture of ref bindings / non-`Static` values is legal |
| [E494](../error-codes.md#e488e499--references--escape-checking) | a view-kind closure would leave its frame; note suggests `escaping` / `consuming` |
| [E500](../error-codes.md#e500e507--moves--ownership) | use after an owning capture moved a non-Copyable; second call of a `consuming` closure |
| [E503](../error-codes.md#e500e507--moves--ownership) | owning capture of a non-Copyable value the frame only borrows (nothing durable to own) |
| [E506](../error-codes.md#e500e507--moves--ownership) | moving a capture out of a normal / `mutating` / `escaping` body (lifted in `consuming`) |
| [E507](../error-codes.md#e500e507--moves--ownership) | the freeze rule: destroying a viewed place, or letting a view outlive it |
| [E603](../error-codes.md#e600e614-e623--closures-externs--declaration-shape) | assigning to a capture in a **normal** body (lifted in `mutating` / `consuming` / `escaping`) |
| [E624](../error-codes.md#e624e625--closure-kinds) | passing-table rejection; owning capture of a frame-provenance value |
| [E625](../error-codes.md#e624e625--closure-kinds) | a `mutating`/`consuming`-kind parameter declared with the wrong access mode |
