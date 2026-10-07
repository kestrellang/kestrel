# Closures

Closures are anonymous functions that capture values from their surrounding scope. They are first-class values: pass them as arguments, store them in variables, fields, and enum payloads, and — with the right type — return them from functions.

A Kestrel closure holds its environment the way its **kind** says it does, and the kind is spelled in the type. There are four:

```kestrel
        (Int64) -> Int64     // normal    — read-only views of the frame
mutating (Int64) -> Int64    // mutating  — writes back to the originals
consuming (Int64) -> Int64   // consuming — owns its captures, runs once
escaping (Int64) -> Int64    // escaping  — owns its captures, shared, outlives the frame
```

Plain `(T) -> U` is right for the vast majority of closures. The keywords appear only when an API needs write-back, hand-off, or storage beyond the frame. You never write a kind on the closure *literal* — the expected type picks the kind, and the literal is built for it.

For the full semantics (drop order, provenance, the exact freeze algorithm), see [the memory model's closure chapter](../memory-model/closures.md).

## Basic Syntax

### No Parameters

The simplest closure has no parameters and no `in` keyword:

```kestrel
let f: () -> Int64 = { 42 };
f()  // Returns 42
```

With explicit empty parameters and the `in` keyword:

```kestrel
let f: () -> Int64 = { () in 42 };
```

### Single Parameter

With explicit type annotation:

```kestrel
let double: (Int64) -> Int64 = { (x: Int64) in x * 2 };
```

With inferred type (requires context):

```kestrel
let double: (Int64) -> Int64 = { (x) in x * 2 };
```

### Multiple Parameters

```kestrel
let add: (Int64, Int64) -> Int64 = { (x: Int64, y: Int64) in x + y };
let add: (Int64, Int64) -> Int64 = { (x, y) in x + y };          // inferred
let f:   (Int64, String) -> Int64 = { (x: Int64, y) in x };      // mixed
```

## Implicit `it` Parameter

When a closure has exactly one parameter and the expected type is known, use the implicit `it` parameter instead of declaring one:

```kestrel
func apply(f: (Int64) -> Int64, x: Int64) -> Int64 {
    f(x)
}

let result = apply({ it * 2 }, 21);  // Returns 42
```

### Rules for `it`

- `it` is only available when the expected function type has exactly 1 parameter
- Using `it` when arity is 0 or 2+ is an error (reported by the solver as **E108**)
- Explicit parameters shadow `it` — you cannot use both
- `it` belongs to the innermost enclosing closure **written without a parameter list**. A nested closure that declares its parameters (`{ (y) in … }`) is transparent: an `it` inside it is the enclosing closure's.
- When a closure's `it` hides another `it` in scope, it still refers to the closure's own parameter, and the compiler warns: **E142** if the hidden `it` belongs to an enclosing closure, **E143** if it is any other binding (a `let it`, a parameter named `it`). A closure never captures an outer binding named `it` through the implicit parameter.

```kestrel
// ERROR[E108]: it used but arity is 0
let f: () -> Int64 = { it };

// ERROR[E108]: it used but arity is 2
let g: (Int64, Int64) -> Int64 = { it };

// ERROR: it not available with explicit params
let h: (Int64) -> Int64 = { (x) in it };

// `it` reaches through a closure that declares its parameters
let counts = xs.map { ys.filter(where: { (y) in y == it }).count };  // `it` is the map element

// WARNING[E142]: nested it shadows the outer closure's it
func apply(f: (Int64) -> Int64) -> Int64 {
    f(10)
}

let f: (Int64) -> Int64 = {
    let outer = it;
    apply({ it + outer })  // inner `it` is a different parameter
};

// WARNING[E143]: the closure's it shadows the outer `let it`
let it = 100;
let r = xs.map { it + 1 };  // `it` is the element, not 100
```

## Trailing Closure Syntax

When a closure is the last argument to a function, write it outside the parentheses (and drop its label):

```kestrel
func apply(f: () -> Int64) -> Int64 { f() }

apply { 42 }                    // instead of apply({ 42 })
```

With other arguments before it:

```kestrel
func fold(initial: Int64, f: (Int64, Int64) -> Int64) -> Int64 {
    f(initial, 10)
}

fold(0) { (acc, n) in acc + n }
```

Combined with `it`, this is the idiomatic collection style:

```kestrel
let numbers = [1, 2, 3];
let doubled = numbers.map { it * 2 };          // [2, 4, 6]

var total: Int64 = 0;
[1, 2, 3, 4].iter().forEach { total = total + it };   // total == 10
```

## Multi-Statement Closures

Closures can contain multiple statements. The last expression is the result:

```kestrel
let compute: (Int64, Int64) -> Int64 = { (x, y) in
    let sum = x + y;
    let doubled = sum * 2;
    doubled + 1
};
```

They support the full statement language:

```kestrel
// Local mutable variables
let process: (Int64) -> Int64 = { (x) in
    var acc = 0;
    acc = acc + x;
    acc = acc + x;
    acc
};

// if expressions
let absolute: (Int64) -> Int64 = { (x) in
    if x > 0 { x } else { -x }
};

// while loops
let sumTo: (Int64) -> Int64 = { (n) in
    var i = 0;
    var sum = 0;
    while i < n {
        sum = sum + i;
        i = i + 1;
    }
    sum
};
```

## The Four Kinds

A function type may carry a kind prefix:

```
fn_type ::= ['mutating' | 'consuming' | 'escaping'] '(' param_types ')' '->' type
```

| kind | captures | body may | callable | copy class | leaves the frame |
|---|---|---|---|---|---|
| **normal** — `(T) -> U` | views of the frame | read | many times, from a `let` | Copyable | no |
| **`mutating`** — `mutating (T) -> U` | `&mutating` views | write back to the originals | many times; calls are exclusive (needs a `var`) | not Copyable | no |
| **`consuming`** — `consuming (T) -> U` | owned (moved / copied in) | move captures out | exactly once — the call consumes it | not Copyable | yes |
| **`escaping`** — `escaping (T) -> U` | owned snapshots, **shared** | read and mutate its own state | many times, from a `let` | Cloneable (clone = share) | yes |

The prefix is a normal part of the type, so it appears anywhere a function type does: `let` annotations, parameters, return types, struct fields, generic arguments, and protocol requirements.

```kestrel
let f: escaping () -> Int64 = { 7 };                   // let annotation
func store(f: escaping () -> Int64) { }                // parameter
func make() -> escaping () -> Int64 { { 7 } }          // return type
struct Button { let onClick: escaping () -> () }       // field
var fns = Array[escaping () -> Int64]();               // generic argument
```

### Which kind should I use?

| you want to… | use |
|---|---|
| filter, transform, compare, or visit — the callback just reads | **normal** (write nothing) |
| let the callback update a variable in the calling function | **`mutating`** |
| return a closure, or store one in a field that outlives the call | **`escaping`** |
| hand a resource to a callback that runs at most once | **`consuming`** |

Start with normal. Reach for a keyword when the compiler tells you to — E603 points at `mutating`, and E494 points at `escaping` / `consuming`.

## Capture Semantics

### Normal Closures Capture Views

> **This changed.** Normal closures used to snapshot their captures. They now hold **views** — live references into the enclosing frame — exactly like a named reference binding (`let r = &x;`). Reads see later writes.

```kestrel
var x = 10;
let f = { x };   // live view of x, not a copy
x = 20;
f()              // 20  (was 10 under the old snapshot model)
```

Nothing is copied or moved at creation time, so building a view closure is free. Copies of a normal closure share the same views:

```kestrel
let k = 7;
let f = { k + 1 };
let g = f;       // normal closures are Copyable
g()              // 8 — same views as f
```

A view over an immutable `let` is indistinguishable from a snapshot, so most existing code is unaffected. The difference only shows when the captured place is written afterwards.

Normal captures are **read-only**. Assigning to one — including a projection like `c.n = 5` — is E603:

```kestrel
var total: Int64 = 0;
let f: () -> Int64 = {
    total = total + 1;   // ERROR[E603]: cannot assign to captured variable 'total'
    total
};
```

> `error[E603]` … *a normal closure captures read-only views; give it a `mutating` expected type (e.g. `mutating () -> ()`) to write back to the original, or fold the value and return it instead*

### Owning Kinds Capture Snapshots

A `consuming` or `escaping` environment must own its contents, so each captured place is copied, cloned, or moved **at creation time**:

```kestrel
var x = 10;
let g: escaping () -> Int64 = { x };   // snapshots x == 10
x = 20;
g()                                     // 10 — the source is untouched
```

| captured place is | owning capture does | source afterwards |
|---|---|---|
| Copyable | bit-copy | untouched |
| Cloneable | `clone()` | untouched |
| non-Copyable, owned by the frame | **move** | dead — later use is E500 |
| carries frame provenance (a view closure, a ref binding) | rejected — E624 | — |

Remember the one-line rule: **view kinds see later writes; owning kinds are snapshots.**

### Captures Are Place-Based

The compiler captures the narrowest *place* the body actually uses, widening only when it must:

```kestrel
{ self.data }          // captures the place `self.data`, not all of `self`
{ self.data.count }    // captures just `self.data.count`
{ self.method() }      // needs the receiver → widens to `self`
{ x + x.f }            // overlapping places merge → one capture of `x`
```

So a closure inside a method on a non-`Copyable` type can use the receiver's fields freely without touching `self`.

### Capturing Non-Copyable Values

A view kind does **not** move a non-`Copyable` capture — it aliases the original's storage, so the source stays live:

```kestrel
struct Res: not Copyable {
    var id: Int64
    func peek() -> Int64 { self.id }
    deinit { }
}

let r = Res(id: 7);
let f = { () in r.peek() };   // view of `r` — nothing is moved
f();                          // 7
r.peek();                     // 7 — `r` is still live
f();                          // 7 again
```

An **owning** kind does move it, and the original is dead afterwards:

```kestrel
let r = Res(id: 7);
let g: escaping () -> Int64 = { r.peek() };   // moves `r` into the environment
r.id   // ERROR[E500]: use of moved value 'r'
```

A multi-call body — normal, `mutating`, or `escaping` — owns at most one copy of each capture, so moving a capture *out* of the body is E506:

```kestrel
func runNormal(f: () -> Res) -> Int64 { f().id }

let a = Res(id: 7);
runNormal({ () in a })   // ERROR[E506]: cannot move captured value 'a' out of a closure
```

Only a `consuming` body, which runs at most once, may move its captures out.

### The Freeze Rule (E507)

A view must not dangle. While a normal or `mutating` closure that views a place is still live, that place is **frozen against destruction**: it cannot be moved, passed to a `consuming` parameter, or destroyed with `deinit x;`.

```kestrel
func sink(consuming r: Res) { }

let r = Res(id: 1);
let f = { r.id };   // views `r.id`
sink(r);            // ERROR[E507]: cannot consume 'r.id' while a closure capturing it is live
f();
```

Three things to know about "frozen":

1. **Plain reassignment stays legal.** Writing to the place is a write *through* a live view, which references already permit. Only destruction is blocked.

   ```kestrel
   struct P { var a: Int64; var b: Int64 }

   var p = P(a: 1, b: 2);
   let f = { p.a };
   p = P(a: 10, b: 20);   // fine — a write, not a destruction
   f();                   // 10 — the view reads the new value
   p.a = 30;
   f();                   // 30
   ```

2. **It is place-granular.** Capturing `self.data` freezes `self.data`, not all of `self`. (One exception today: a capture reached through a *nested* closure widens to the whole enclosing local, so the freeze is coarser there.)

3. **It is lexical.** The freeze runs to the end of the scope of every binding that may carry the view — including struct fields and copies of the closure. Once those scopes end, the place thaws:

   ```kestrel
   let r = Res(id: 7);
   if cond {
       let f = { r.id };   // freeze starts here…
       f();
   }                       // …and ends with this block
   sink(r);                // legal again
   ```

The rule has a second half: a value carrying a view may not be stored into a binding that **outlives** the captured place, even if nothing is ever destroyed.

```kestrel
var g: () -> Int64 = { () in 0 };   // capture-free literal: carries no view
if cond {
    let n: Int64 = 9;
    g = { n };   // ERROR[E507]: a closure viewing 'n' cannot outlive 'n'
}                // `n` dies here; `g` would dangle
g()
```

Owning kinds freeze nothing — their captures are theirs.

### Parameter Shadowing

Closure parameters shadow captured variables with the same name:

```kestrel
let x = 100;
let f: (Int64) -> Int64 = { (x) in x + 20 };
f(22)  // Returns 42 — uses the parameter (22), not the captured x (100)
```

### Loop Captures

A view-kind closure over something declared inside a loop body is only valid for that iteration, and storing it somewhere longer-lived is E507:

```kestrel
var g: () -> Int64 = { () in 0 };
for i in 1..=3 {
    let n = i * 10;
    g = { n };   // ERROR[E507]: a closure viewing 'n' cannot outlive 'n'
}
```

Collecting closures for later use is exactly what the owning kinds are for. Give the container an `escaping` element type and each iteration snapshots its own values:

```kestrel
var fns = Array[escaping () -> Int64]();
for i in 1..=3 {
    fns.append({ i * 10 });   // the element type supplies the escaping kind
}
(fns(0))();   // 10 — still valid after the loop ended
(fns(2))();   // 30
```

## `mutating`: Writing Back

A `mutating` closure captures `&mutating` views, so its assignments hit the enclosing variables:

```kestrel
var total: Int64 = 0;
var count: Int64 = 0;
var bump: mutating (Int64) -> () = {
    total = total + it;
    count = count + 1;
};
bump(5);
bump(7);
// total == 12, count == 2 — the writes went back
```

Two consequences:

- Calling one is an **exclusive** use, so it must live in a `var` (or a `mutating` parameter). Calling a `let`-held `mutating` closure is E203.
- The value is **not Copyable** — that is what keeps write-back sound without aliasing analysis. Binding it to a second name is a move.

Like a `mutating` method, it cannot leave its frame (E494).

The standard library uses `mutating` for its eager side-effecting callbacks — `Iterator.forEach` / `tryForEach`, and `inspect` / `inspectErr` on `Optional` and `Result` — so the accumulator idiom just works:

```kestrel
var total: Int64 = 0;
[1, 2, 3, 4].iter().forEach { total = total + it };
// total == 10
```

A read-only closure value passes into a `mutating` parameter too (see [the passing table](#passing-closures-around)), so an existing `let`-bound callback keeps working:

```kestrel
public var log: Int64 = 0;

let record: (Int64) -> () = { (x) in log = log * 10 + x };
[1, 2, 3].iter().forEach(record);
[4].iter().forEach(record);          // still usable — the adapter didn't consume it
```

There is no exclusivity rule: two `mutating` closures over the same place are permitted.

## `escaping`: Closures That Outlive the Frame

An `escaping` closure owns snapshots of its captures in a **shared heap environment**. It is the kind that makes closure factories work:

```kestrel
func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }   // snapshots count; mutates its own copy
}

let next = makeCounter(10);
next();   // 11
next();   // 12
```

### Reference Semantics

An `escaping` closure is **Cloneable, not bitwise-Copyable**: duplicating a handle *shares* the environment rather than duplicating it. Copies see one another's mutations. This is the deliberate exception to Kestrel's value semantics, and the keyword marks it.

```kestrel
let next = makeCounter(10);
next();             // 11
let alias = next;   // shares the environment
alias();            // 12
next();             // 13 — one counter, two handles
```

### Storing Closures in Fields

An `escaping` field keeps a struct storable long-term, and the aggregate copy rules follow: the struct becomes Cloneable and copying it shares the handle.

```kestrel
struct Button {
    let onClick: escaping () -> Int64
}

func makeButton() -> Button {
    var clicks = 0;
    Button(onClick: { clicks = clicks + 1; clicks })
}

let b = makeButton();     // callable long after makeButton returned
let b2 = b;               // struct copy shares (retains) the environment
(b.onClick)();            // 1
(b2.onClick)();           // 2 — the alias observes b's mutation
```

A field of *normal* kind is fine for short-lived helper structs, but it makes the whole struct frame-bound — it cannot be returned or stored beyond the frame.

### Cleanup and Cycles

Captures in an acyclic environment are dropped exactly once, at the last release — so a captured resource's `deinit` runs at a predictable point:

```kestrel
func makeReader() -> escaping () -> Int64 {
    let r = Res(id: 7);
    { r.peek() }         // `r` is moved into the environment
}
// when the last handle dies, `r.deinit` runs — exactly once
```

**Caveat:** the environment is refcounted, and refcounting does not collect strong cycles. A cycle of escaping environments leaks, and its captures are never deinitialized. Break such cycles explicitly until weak references exist.

## `consuming`: One-Shot Hand-Off

A `consuming` closure owns its captures in a unique (unshared) environment. Calling it **consumes** it, so a second call is an ordinary use-after-move:

```kestrel
func runTwice(consuming f: consuming () -> Int64) -> Int64 {
    let a = f();
    let b = f();   // ERROR[E500]: use of moved value 'f'
    a + b
}
```

Because it runs at most once, its body is the one place allowed to move captures *out* — E506 is lifted there. That makes it the kind for transferring a resource into a callback:

```kestrel
func transfer(consuming r: Res) { }
func onDone(consuming f: consuming () -> ()) { f(); }

let file = Res(id: 3);
onDone({ () in transfer(file) });   // `file` moves in, then out — deinit runs exactly once
```

It owns everything it holds, so it may leave the frame:

```kestrel
func makeThunk(n: Int64) -> consuming () -> Int64 {
    { n + 1 }
}

let t = makeThunk(41);
t();   // 42
```

`consuming` closures are not Copyable — handing one around is a move — and a `consuming` closure that is never called still drops its environment, dropping every capture exactly once.

## Capture-Free Closures Satisfy Every Kind

A closure with no captures — or a named function used as a value — is a bare function pointer. It has no environment, never allocates, satisfies **every** kind, and escapes freely.

```kestrel
func seven() -> Int64 { 7 }

func constant() -> () -> Int64 {
    { 42 }                                  // no captures: freely returnable
}

let cb: escaping () -> Int64 = { 7 };       // no allocation; null environment
var m:  mutating () -> Int64 = seven;
let c:  consuming () -> Int64 = seven;
```

## Passing Closures Around

The expected type declares both how the callee may call the closure and what ownership it receives. A **literal** is built directly for the expected kind. For an existing closure **value**:

| have ↓ \ expected → | normal | `mutating` | `consuming` | `escaping` |
|---|---|---|---|---|
| normal | ✓ | ✓ | ✗ a frame view is not owned | ✗ frame-bound |
| `mutating` | ✗ | ✓ | ✗ a frame view is not owned | ✗ frame-bound |
| `consuming` | ✗ | ✗ | ✓ | ✗ one-shot |
| `escaping` | ✓ | ✗ shared, not exclusive | ✓ | ✓ |

A rejected cell is **E624**, with a note naming the property the source cannot supply. The two you are most likely to hit:

```kestrel
func store(f: escaping () -> Int64) { }

var x: Int64 = 1;
let view = { x };
store(view);
// error[E624]: closure kind mismatch: expected an 'escaping' closure, found a normal closure
//            = a frame-view closure is frame-bound and can never flow into an 'escaping' slot
```

```kestrel
func callTwice(f: (Int64) -> ()) { f(1); f(2); }

var bump: mutating (Int64) -> () = { total = total + it; };
callTwice(bump);   // error[E624] — weakening an exclusive-call value to a shared slot
```

The fix in both cases is usually to write the literal directly against the expected type instead of routing it through a `let`:

```kestrel
store({ x });   // OK — built as an owning literal, snapshotting x
```

An owning literal also refuses to launder frame-bound state into owned storage: capturing a ref binding (`let r = &x;`) in an `escaping` or `consuming` literal is E624. Copy the referenced value into a `let` first and capture that.

A conversion never recompiles the body or grants it new capture powers. Normal → `mutating` is an exclusive-call adapter over the same frame views, and `escaping` → normal produces a non-owning *view* of the shared environment (frame-bound, rooted at the original handle):

```kestrel
func callNormal(f: () -> Int64) -> Int64 { f() }

let next = makeCounter(0);
callNormal(next);   // 1 — the callee drives the same shared environment
next();             // 2 — the caller's handle still sees it
```

### Kind and Access Mode Must Agree

The kind also dictates the parameter access mode needed to call it: a `mutating`-kind parameter must be declared `mutating`, and a `consuming`-kind parameter `consuming`. A mismatch is **E625**:

```kestrel
func each(action: mutating (Int64) -> ()) { }
// error[E625]: a 'mutating' closure parameter must have the 'mutating' access mode

func each(mutating action: mutating (Int64) -> ()) { }   // OK
```

The two keywords sit next to each other in the `consuming` case — the first is the parameter's access mode, the second is the closure's kind:

```kestrel
func onDone(consuming f: consuming () -> ()) { f(); }
//          ^^^^^^^^^    ^^^^^^^^^
//          access mode  closure kind
```

## Parameter Conventions

A closure's *parameters* carry access modes just like a function's, and this axis is **independent of the closure's kind**. `(mutating T) -> R` says the callback mutates its argument; it says nothing about how the closure captures.

The convention may be written on the literal or inferred from the expected type — including a `let` annotation:

```kestrel
struct Counter { var n: Int64 }

func apply(mutating c: Counter, with f: (mutating Counter) -> Int64) -> Int64 {
    f(c)
}

let f: (mutating Counter) -> Int64 = { (x) in x.n = x.n + 3; x.n };
let g = f;                    // still normal kind: Copyable, `let`-bound

var c = Counter(n: 10);
apply(c, with: f);            // 13
apply(c, with: g);            // 16 — c.n is now 16
```

The convention can also be written explicitly on the literal:

```kestrel
apply(c, with: { (mutating x: Counter) in x.n = x.n + 3; x.n });
```

Conventions are checked contravariantly: a closure whose parameter is `mutating` cannot be passed where a plain (borrowing) parameter is expected. Only `mutating` is spellable inside a function type today — `consuming` parameter conventions in function types are not yet supported.

Closure parameters are immutable by default — assigning to one is E604:

```kestrel
let f: (Int64) -> Int64 = { (x) in
    x = 10;   // ERROR[E604]: cannot assign to closure parameter 'x'
    x
};
```

Use a local mutable variable instead:

```kestrel
let f: (Int64) -> Int64 = { (x) in
    var temp = x;
    temp = temp * 2;
    temp
};
```

## `return` Inside a Closure

`return` in a closure body returns **from the closure**, not from the enclosing function:

```kestrel
func callTwice(f: () -> Int64) -> Int64 { f() + f() }
func pick(n: Int64, f: (Int64) -> Int64) -> Int64 { f(n) }

callTwice({ return 5 });   // 10 — both calls run; the caller is not diverted

pick(3, { (x) in
    if x > 0 { return x * 10 };
    -1
});   // 30 — an early return plus a fallthrough tail, both closure-local
```

## Type Inference

Kestrel infers closure types from context. Type information flows from:

1. **The expected type** — function parameter, variable annotation, return type, field type
2. **The closure body** — the result type comes from the trailing expression

```kestrel
// Parameter types inferred from the expected type
let f: (Int64) -> Int64 = { (x) in x + 1 };

// Return type inferred from the body
let g: (Int64) -> Int64 = { (x: Int64) in x * 2 };

// Both inferred from the call site
func transform(x: Int64, f: (Int64) -> Int64) -> Int64 { f(x) }
transform(5, { (x) in x * 2 });

// ERROR[E606]: cannot infer without context
let h = { (x) in x };
```

The expected type also supplies the **kind**. An expected normal type is never silently upgraded — if the body writes to a capture, you get E603 rather than an implicit `mutating`. Without any expected type, a read-only body infers normal, assignment through a capture infers `mutating`, and moving a capture out infers `consuming`; escaping is never inferred, so the marker stays honest.

### Type Inference with `it`

`it`'s type comes from the expected function type:

```kestrel
func apply(f: (Int64) -> Int64, x: Int64) -> Int64 { f(x) }

apply({ it * 2 }, 21);   // `it` is Int64
```

## Closures as Values

### Stored in Variables

```kestrel
let f: (Int64) -> Int64 = { it * 2 };
f(21)  // 42
```

### Passed as Arguments

```kestrel
func apply(x: Int64, f: (Int64) -> Int64) -> Int64 { f(x) }

apply(10, { it + 1 })  // 11
```

### Returned from Functions

A capture-free closure returns at any kind. A **capturing** closure must be returned at an owning kind — write `escaping` (shared, multi-call) or `consuming` (unique, one-shot) in the return type:

```kestrel
func makeTripler() -> (Int64) -> Int64 {
    { (x) in x * 3 }            // OK: captures nothing
}

func makeMultiplier(n: Int64) -> escaping (Int64) -> Int64 {
    { (x) in x * n }            // OK: owned snapshot of `n`
}

func makeAdder(n: Int64) -> (Int64) -> Int64 {
    { it + n }                  // ERROR[E494]: captures local `n`, which does
                                //              not outlive the call
}
```

The E494 diagnostic tells you the fix directly:

> *a normal or `mutating` closure holds VIEWS into this frame, so it cannot leave it*
> *write an owning kind in the expected/return type — `escaping (…) -> …` (shared environment, callable many times) or `consuming (…) -> …` (unique environment, called once) — and the literal is rebuilt with owned captures*

The check is provenance-based, not syntactic, so laundering a view closure through a `let` binding or a struct field is caught too:

```kestrel
struct Holder { var f: () -> Int64 }

func make() -> Holder {
    let n: Int64 = 41;
    Holder(f: { () in n + 1 })   // ERROR[E494] — tracked through the field
}
```

### Stored in Structs

```kestrel
struct Handler {
    let action: (Int64) -> Int64
}

let h = Handler(action: { it * 2 });
(h.action)(21)  // 42
```

Parentheses around the field access are required when calling: `(h.action)(arg)`.

A normal-kind field makes the struct frame-bound. Use `escaping` for a struct that is returned or stored:

```kestrel
struct Handler {
    let action: escaping (Int64) -> Int64
}
```

### Stored in Enums

```kestrel
enum Action {
    case Transform(f: (Int64) -> Int64)
    case NoOp
}

let action = Action.Transform(f: { it * 2 });

match action {
    .Transform(f: f) => f(21),
    .NoOp => 0
}
```

### Generic Containers

```kestrel
struct Provider[T] {
    let provide: () -> T
}

let p = Provider[Int64](provide: { 42 });
(p.provide)()  // 42
```

Kinds participate in type identity, so `Array[escaping () -> Int64]` and `Array[() -> Int64]` are different types, and a protocol requirement's kind must match its witness exactly:

```kestrel
protocol Runner {
    func runOnce(consuming f: consuming () -> Int64) -> Int64
    func runEach(mutating f: mutating (Int64) -> ())
    func store(f: escaping () -> Int64) -> Int64
}
```

## Nested Closures

Closures can contain other closures. An inner closure may capture from the outer one, as long as it doesn't escape the frame it was created in:

```kestrel
func apply(f: () -> Int64) -> Int64 { f() }

// OK: the inner closure captures `x` but is only passed down
let f: (Int64) -> Int64 = { (x) in
    apply({ x + 1 })
};
```

Currying needs an owning inner kind, because the inner closure is *returned* out of the outer closure's frame:

```kestrel
// ERROR[E494]: the inner closure captures `x` and escapes the outer frame
let g: (Int64) -> (Int64) -> Int64 = { (x) in { (y) in x + y } };

// OK once the inner type owns its captures
func adderFactory(x: Int64) -> escaping (Int64) -> Int64 {
    { (y) in x + y }
}
```

An escaping closure's body may still build ordinary view closures — they are rooted in the *call's* frame, not the environment:

```kestrel
func makeAccumulator(start: Int64) -> escaping (Int64) -> Int64 {
    var total = start;
    { (n) in
        let double = { n * 2 };      // nested view closure: this call's frame
        total = total + double();
        total
    }
}
```

### Nested `it` Shadowing

Each closure written without a parameter list has its own `it`; the innermost one wins, and hiding an outer `it` is warned about (E142):

```kestrel
func apply(f: (Int64) -> Int64) -> Int64 { f(5) }

let f: (Int64) -> Int64 = {
    let outer = it;           // outer closure's it
    apply({ it + outer })     // WARNING[E142]: inner closure's it is different
};
```

Name the parameter to make the intent explicit and silence the warning: `apply({ (x) in x + outer })`.

## Immediate Invocation

Closures can be invoked immediately where they're defined:

```kestrel
let x = { 42 }();                                    // 42
let sum = { (x: Int64, y: Int64) in x + y }(10, 20); // 30

let result = {
    let a = 10;
    let b = 20;
    a + b
}();   // 30 — a and b are not visible outside
```

## Type Checking

The compiler validates closure types against the expected type:

```kestrel
// ERROR[E601]: arity mismatch — too few parameters
let f: (Int64, Int64) -> Int64 = { (x) in x };

// ERROR[E601]: arity mismatch — too many parameters
let g: (Int64) -> Int64 = { (x, y) in x + y };

// ERROR: return type mismatch
let h: (Int64) -> String = { (x) in x * 2 };

// ERROR[E109]: parameter type mismatch (E602 is reserved and not implemented)
let i: (Int64) -> Int64 = { (x: String) in 42 };

// ERROR: closure assigned to non-function type
let j: Int64 = { 42 };
```

## Higher-Order Functions

### Composition

Composition builds a closure over its arguments and returns it, so both the parameters and the result are `escaping`:

```kestrel
func compose(
    f: escaping (Int64) -> Int64,
    g: escaping (Int64) -> Int64
) -> escaping (Int64) -> Int64 {
    { (x) in g(f(x)) }
}

let add10: escaping (Int64) -> Int64 = { it + 10 };
let double: escaping (Int64) -> Int64 = { it * 2 };
let composed = compose(add10, double);
composed(11)  // (11 + 10) * 2 = 42
```

### Apply Twice

A callback that is only *called*, never stored, stays normal:

```kestrel
func applyTwice(f: (Int64) -> Int64, x: Int64) -> Int64 {
    f(f(x))
}

applyTwice({ (x) in x + 10 }, 22)  // 42
```

That split is the stdlib's rule of thumb too: eager operations (`fold`, `sorted(by:)`, `any(where:)`, `first(where:)`) take normal closures; lazy builders that store the callback (`Iterator.map`, `filter`, `filterMap`, `scan`, `Str.split(where:)`) take `escaping` ones.

## Common Patterns

### Factory Functions

```kestrel
func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in
        count = count + 1;
        count
    }
}

let next = makeCounter(0);
next();  // 1
next();  // 2
```

### Callbacks

```kestrel
struct Button {
    let onClick: escaping () -> ()
}

let button = Button(onClick: {
    println("Button clicked!");
});
```

### Configuration

```kestrel
struct Builder { var width: Int64; var height: Int64 }

func configure(mutating b: Builder, f: (mutating Builder) -> ()) {
    f(b);
}

var myBuilder = Builder(width: 0, height: 0);
configure(myBuilder) { (b) in
    b.width = 100;
    b.height = 200;
}
```

(The closure parameter is unlabeled here — a trailing closure only binds to an unlabeled parameter. For a labeled one such as `Iterator.filter(where:)`, spell the label: `filter(where: { it > 0 })`.)

### Accumulating

```kestrel
var total: Int64 = 0;
[1, 2, 3, 4].iter().forEach { total = total + it };
```

### Resource Hand-Off

```kestrel
func onDone(consuming f: consuming () -> ()) { f(); }

let file = openFile();
onDone({ () in close(file) });   // `file` moves into the closure, then out
```

## Grammar

```
closure ::= '{' closure_params? body '}'

closure_params ::= '(' param_list ')' 'in'
                 | '(' ')' 'in'

param_list ::= param (',' param)*

param ::= 'mutating'? identifier (':' type)?

body ::= statement* expression?
       | expression

fn_type ::= closure_kind? '(' param_types ')' '->' type

closure_kind ::= 'mutating' | 'consuming' | 'escaping'

// Note: when no closure_params are given and the body uses `it`,
// the implicit single-parameter form is used.
```

### Syntax Notes

- No `in` keyword when there are no explicit parameters; empty `()` requires `in`
- Parameters can mix typed and untyped forms
- The body is a block that can contain statements and a trailing expression
- There is **no kind on the literal** — the kind comes from the expected type
- Parenthesizing a kinded function type preserves the kind: `(mutating () -> ())` is still the mutating kind
- There is no explicit return-type annotation syntax; the return type is inferred

## Diagnostics You May Hit

| Code | Meaning |
|---|---|
| [E203](../error-codes.md#e200e211--mutability-access-modes--assignment) | Calling a `mutating`-kind closure held in a `let` — calls are exclusive, so hold it in a `var` |
| [E494](../error-codes.md#e480e499--references--escape-checking) | A view-kind closure (or a value carrying one) would leave its frame — use `escaping` / `consuming` |
| [E500](../error-codes.md#e500e507--moves--ownership) | Use after an owning capture moved a non-`Copyable`; also a second call of a `consuming` closure |
| [E503](../error-codes.md#e500e507--moves--ownership) | Owning capture of a non-`Copyable` value the frame only borrows |
| [E506](../error-codes.md#e500e507--moves--ownership) | Moving a capture out of a normal / `mutating` / `escaping` body (lifted in `consuming`) |
| [E507](../error-codes.md#e500e507--moves--ownership) | The freeze rule: destroying a viewed place, or letting a view outlive it |
| [E108](../error-codes.md#e100e141--type-checking-names-parameters--literals) | `it` used where the expected arity isn't 1 — caught by the solver (the E600 descriptor is reserved and never fires) |
| [E601](../error-codes.md#e600e614-e623--closures-externs--declaration-shape) | Closure parameter count doesn't match the expected type |

| [E603](../error-codes.md#e600e614-e623--closures-externs--declaration-shape) | Assigning to a capture in a normal body — the note points at `mutating` |
| [E604](../error-codes.md#e600e614-e623--closures-externs--declaration-shape) | Assigning to a closure parameter |
| [E606](../error-codes.md#e600e614-e623--closures-externs--declaration-shape) | Could not infer a closure parameter type — add an annotation or context |
| [E624](../error-codes.md#e624e625--closure-kinds) | Passing-table rejection; also an owning capture of a frame-provenance value |
| [E625](../error-codes.md#e624e625--closure-kinds) | A `mutating` / `consuming`-kind parameter declared with the wrong access mode |

## Limitations

- **Recursive closures** (`let f = { ... f(...) ... }`) are not expressible — the binding is not in scope inside its own initializer. Use a named function.
- **No capture lists.** There is no Swift-style `[x]` capture list and no kind-on-literal syntax; the expected type is the only control.
- **No explicit return-type annotation** on a literal.
- **`consuming` parameter conventions inside function types** (`(consuming T) -> R`) are not yet supported; only `mutating` is.
- **Nested-closure captures widen** to the whole enclosing local rather than the narrowest place, which makes the freeze rule coarser inside nested closures.
- **Strong cycles between `escaping` environments leak** — refcounting does not collect them, and the captures are never deinitialized.

## See Also

- [Closures and Capture](../memory-model/closures.md) — the memory-model view: capture modes, drop order, provenance, the full diagnostics table
- [Closure Semantics](../design/closures.md) — the design document behind the four kinds
- [Functions](functions.md) — parameter access modes (`mutating` / `consuming`) and function types
- [References](references.md) — `&T`, provenance, and the escape rule that view closures share
