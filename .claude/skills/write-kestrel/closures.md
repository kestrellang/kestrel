# Closures

Full reference: `docs/language/closures.md`.

```kestrel
let add = { (a: Int64, b: Int64) in a + b };
let double: (Int64) -> Int64 = { it * 2 };     // single param → `it`
numbers.map { it * 2 }                          // trailing closure
```

## Closure Kinds

A function type may carry a **kind** prefix. The kind decides how the closure holds its
captures, how it can be called, and whether it can leave the frame. Kinds are spelled
**on types only — never on a literal**; a literal is built for whatever kind its expected
type asks for.

```kestrel
(Int64) -> Int64             // normal (default) — read-only VIEWS of the frame
mutating (Int64) -> ()       // &mutating views — assignments write back to the caller
consuming () -> File         // owns its captures; called exactly once; may move them out
escaping () -> Int64         // owns snapshots in a shared heap env; may outlive the frame
```

| kind | captures | callable | copy class | can leave frame |
|---|---|---|---|---|
| normal | views (see later writes) | many times, from `let` | Copyable | no (E494) |
| `mutating` | `&mutating` views | many times, needs `var` | `not Copyable` | no |
| `consuming` | owned; body may move them **out** | exactly once | `not Copyable` | yes |
| `escaping` | owned snapshots, **shared** | many times, from `let` | Cloneable (clone = retain) | yes |

Which to reach for: **normal** for predicates/transforms/visitors — the default and ~86% of
the stdlib; **`mutating`** when the callback writes back into caller variables (`forEach`);
**`escaping`** when the callee stores or returns the callback (lazy adapters, struct fields);
**`consuming`** for a one-shot hand-off that transfers a resource.

A kind goes anywhere a function type goes — params, return types, `let` annotations, struct
fields, protocol requirements, type aliases. `escaping` is a contextual keyword (still usable
as an identifier); `mutating`/`consuming` are reserved.

```kestrel
func each(mutating action: mutating (Int64) -> ()) { }        // kind + matching access mode
func store(consuming f: escaping () -> Int64) -> Holder { }
func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }                               // literal built as escaping
}
struct Button { let onClick: escaping () -> Int64 }            // storable long-term
let b2 = b; (b2.onClick)();                                    // shares one environment
```

## Capture Semantics per Kind

Normal/`mutating` capture **views** — later writes are visible. `escaping`/`consuming`
capture **owned snapshots** at creation (bit-copy Copyable, `clone()` Cloneable, **move**
non-Copyable → source dead, E500 on later use).

```kestrel
var x = 10;
let f = { x };                            // view
x = 20;
f()                                        // 20  — NOT a snapshot

var y = 10;
let g: escaping () -> Int64 = { y };      // snapshot of 10
y = 20;
g()                                        // 10
```

`escaping` closures have **reference semantics**: `clone`/copy retains one shared heap
environment, so aliases see each other's mutations, and the captures drop once at last
release. Refcounting does not collect **strong cycles** — a cycle of escaping environments
leaks and its captures are never deinitialized.

## Passing Rules (the ones that bite)

| have ↓ \ expected → | normal | `mutating` | `consuming` | `escaping` |
|---|---|---|---|---|
| normal | ✓ | ✓ (adapter) | ✗ E624 | ✗ E624 |
| `mutating` | ✗ | ✓ | ✗ | ✗ |
| `consuming` | ✗ | ✗ | ✓ | ✗ |
| `escaping` | ✓ (view) | ✗ E624 | ✓ | ✓ |

- A closure **literal** adapts automatically — the expected type retrofits the kind, trailing
  closures included. An already-bound closure **value** does not: `let scale = { x * f };
  iter.map(as: scale)` is **E624**. Fix by annotating the expected kind where the closure is
  created: `let scale: escaping (Int64) -> Int64 = { x * f };` (it is then built with owned
  captures) — never by re-annotating the existing value.
- A `mutating`-kind value must be held in `var` to be called; calling one held in `let` is the
  **E203** family, not E624.
- Kind and access mode must pair: `mutating` kind on a `mutating` param, `consuming` kind on a
  `consuming` param, else **E625**. The kind is independent of a *param's own* convention
  inside the type — `(mutating T) -> R` is still a normal closure.
- Named/capture-free functions satisfy every kind.

## The Freeze Rule (view kinds)

While a live normal/`mutating` closure views a place, that place is frozen against
**destruction** — it cannot be moved, passed to a `consuming` param, or `deinit`ed (**E507**).
Plain reassignment stays legal. A view-carrying value also cannot be stored into a
longer-lived binding than its captures (E507 "cannot outlive").

```kestrel
let r = Res(v: 3);
let f = { r.v };        // views r
sink(r);                // ERROR E507 — consuming a frozen place
```

```kestrel
var g: () -> lang.i64 = { 0 };
if cond {
    let r = Res(v: 9);
    g = { r.v };        // ERROR E507 — the view would outlive `r`
}
```

Assigning to a capture in a **normal** body is **E603** (including through a field,
`c.n = 5`) — the fix is a `mutating` expected type. Moving a capture out is E506 except in a
`consuming` body. Returning/storing a view closure is **E494**; the fix-it is an owning kind
in the expected type.

## Which kind the stdlib asks for

- `mutating`: `Iterator.forEach` / `tryForEach`, `Optional.inspect`,
  `Result.inspect` / `inspectErr`.
- `escaping`: the lazy builders that store their callback
  (`Iterator.map(as:)`, `filter(where:)`, `filterMap`, `flatMap`, `scan`,
  `takeWhile`, `skipWhile`, `inspect`, `intersperseWith`,
  `Slice.split(where:)`, `StringSlice.split(where:)`) and adapter/view inits.
- Normal: everything else, including the eager ones (`fold`, `sorted(by:)`,
  `find(where:)`, `any(where:)`, `Slice.map`, `Slice.filter`).

## Gotchas

- Kinds live on function types only. There is no `{ mutating () in … }`
  literal syntax and no capture list.
- `it` works inside string-interpolation holes: `map { "x \(it)" }`.
- A trailing closure binds only to an unlabeled closure param. When a
  trailing closure fails with "wrong label", spell the label:
  `xs.iter().filter(where: { it > 0 })`.
- Calling a closure held in a field from outside the type needs parens:
  `(h.action)()`. Inside the type, `self.action(x)` works.
- Shorthand params: `{ a, b in a + b }`; typed: `{ (a: Int64, b: Int64) in a + b }`.
