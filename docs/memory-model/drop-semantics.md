# Drop Semantics (RAII)

Kestrel uses RAII (Resource Acquisition Is Initialization) for deterministic resource cleanup through `deinit`.

## The `deinit` Block

Types can define cleanup logic that runs when a value goes out of scope:

```kestrel
struct FileHandle: not Copyable {
    var fd: Int64

    deinit {
        closeFd(self.fd);
    }
}

func example() {
    let f = FileHandle(fd: openFd("file.txt"));
    // use f...
}  // f.deinit called here, file closed
```

Inside `deinit`, `self` is fully valid — fields are dropped *after* the body runs. `deinit` cannot fail (no `throws`) and cannot be called as a method (`f.deinit()` is not callable); use the `deinit` *statement* below for early cleanup.

A type may have at most one `deinit`. Copyable types may declare `deinit` too — but then it runs once per copy, so it must be idempotent-by-value (usually a sign the type should be `not Copyable` instead).

## Drop Order

Local values drop in **reverse order of declaration** at scope exit:

```kestrel
func example() {
    let a = Resource("a");
    let b = Resource("b");
    let c = Resource("c");
}  // Drops: c, b, a
```

This ensures resources that depend on earlier resources are cleaned up first.

## Drop Rules

### 1. Scope Exit

Values are dropped when they go out of scope — including via `return`, `break`, `continue`, and early `guard` exits, which each emit the drops for every scope they leave:

```kestrel
func example(condition: Bool) {
    let f = FileHandle(...);
    if condition {
        let g = FileHandle(...);
        // use g
    }  // g dropped here
    // f still valid
}  // f dropped here
```

### 2. Move Semantics

If a value is **moved**, its `deinit` is **not** called at the source. The new owner is responsible:

```kestrel
func consume(consuming f: FileHandle) {
    // f is owned here
}  // f.deinit called here

func example() {
    let f = FileHandle(...);
    consume(f);  // f is moved
}  // f.deinit NOT called here (already moved)
```

### 3. Conditional Moves Use Drop Flags

When a value is moved on only some control-flow paths, the compiler tracks its state at runtime and drops it only if it still owns a value:

```kestrel
func example(condition: Bool) {
    let f = FileHandle(...);
    if condition {
        consume(f);   // moved on this path only
    }
}  // f dropped here iff !condition (flag-guarded)
```

(Using `f` again after the `if` would be E501 `maybe_moved` — the flag handles *dropping*, not *reuse*.)

### 4. Reassignment Drops the Old Value

Assigning to a `var` (or to a field) drops the previous value first:

```kestrel
var f = FileHandle(fd: 1);
f = FileHandle(fd: 2);   // FileHandle(1).deinit runs here
```

### 5. Early Drop with the `deinit` Statement

A value can be explicitly dropped before scope end with the `deinit` statement:

```kestrel
func example() {
    var f = FileHandle(...);
    // ... use f ...
    deinit f;    // runs f's deinit now
    print("file is closed");
    let x = f;   // ERROR(E500): f was moved by `deinit f;`
}
```

`deinit x;` immediately destroys the value and marks the variable as moved: a second `deinit x;`, or any later use, is a use-after-move error. The operand must be a declared local variable. (There is no `drop(x)` function — early drop is a statement, not an intrinsic call.)

### 6. Temporaries

Unnamed temporaries (`process(FileHandle(...))`) are dropped at the end of the enclosing statement — unless consumed by the call, in which case the callee drops them.

## Struct Field Drop Order

When a struct value is dropped, its **`deinit` body runs first**, then the fields drop in **reverse declaration order**:

```kestrel
public var log: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { log = log * 10 + self.id; }
}

struct Container: not Copyable {
    var first: Res    // dropped last
    var second: Res   // dropped first
    deinit { log = log * 10 + 9; }
}

// Dropping a Container(first: Res(1), second: Res(2)):
//   Container.deinit (9), then second (2), then first (1)  =>  log == 921
```

Enum payloads follow the same rule: only the *active* case's payload is dropped, and multi-field payloads drop in reverse field order.

## Initializers

### Field Reassignment in `init` Drops the Old Value

Init bodies track per-field initialization state (the same definite-initialization lattice as local `var`s). The first assignment to a field initializes it; a *re*assignment drops the previous value; a maybe-initialized field (assigned on only one branch) gets a flag-guarded drop:

```kestrel
struct Holder: not Copyable {
    var a: Res
    init() {
        self.a = Res(id: 1);
        self.a = Res(id: 2);   // Res(1).deinit runs here
    }
}
```

### Partial Initialization and Failure

If a failable init exits with `return null` after initializing only some fields, exactly the initialized fields are dropped — no more, no less:

```kestrel
struct Two: not Copyable {
    var a: Res
    var b: Res
    init(flag: Bool)? {
        self.a = Res(id: 1);
        if flag { return null; }   // drops a only
        self.b = Res(id: 2);
    }
}
```

### Failable-Init Delegation Propagates Failure

An init that delegates (`self.init(...)`) to a failable init propagates the inner failure: the outer init returns `.None` and does not touch the (never-completed) `self`. The inner init unwinds its own partially-initialized fields exactly once.

## Recursive Types

`indirect enum` values (and other heap-indirected recursive structures) drop their reachable payloads recursively when the root drops.

---

## Design Notes

These were open questions in the original design; all are now decided by implementation:

- **`deinit` is infallible.** It cannot `throw` or return a value. For fallible cleanup, provide an explicit `close() -> () throws E` method and keep `deinit` as the last-resort fallback.
- **No unwinding.** `fatalError` / trapping aborts the process immediately; `deinit`s do not run on a panic path. There is no double-panic problem because there is no unwinding.
- **`self` is whole in `deinit`.** All fields are readable; they drop after the body.
- **Field drops are automatic.** A `deinit` body must not (and cannot) manually destroy fields; the compiler appends field drops after the body.
- **No explicit destructor calls** — `f.deinit()` is not invocable; use `deinit f;` or an explicit `close()`-style method.
- **Reference cycles** are only constructible through shared-ownership types (refcounted boxes); plain value types cannot form cycles (self-containment is rejected at declaration, E449/E450). Cycles through shared ownership leak — use weak references at the API level to break them.
