# Kestrel Effects System

This document captures the full design for Kestrel's algebraic effects system.
It supersedes the suspendable functions, generators, async/await, async generators,
swappable runtimes, and implicits sections in `FUTURE_IDEAS.md`.

## Philosophy

Three language features replace a dozen separate mechanisms:

- **Effects** (`throws`, `yields`, `async`) — track behavioral changes in function types
- **`given`** — ambient scoped values (allocators, config, random)
- **`guard`/`catch`** — handle effects at any level

Effects are for things that **change the calling convention**: `throws` changes control
flow, `yields` creates a resumable coroutine, `async` creates a suspendable coroutine.
Things like IO and allocation are NOT effects — IO is just what programs do, and allocation
is handled by `given` instead.

---

## Effects

### Declaration Syntax

Effects appear in function signatures with sugar for the three core effects:

```kestrel
func parse(s: String) -> Int64 throws ParseError
func countdown(from n: Int64) yields Int64
func fetchUser(id: Int64) -> User async throws AppError
func custom() with MyEffect                               // general form
```

| Sugar | General form | Calling convention |
|-------|-------------|-------------------|
| `throws E` | `with Fail[E]` | Non-resumable by default, resumable via handler |
| `yields T` | `with Yield[T]` | Resumable, consumer-driven |
| `async` | `with Async` | Resumable, executor-driven |

### Propagation Rule

If you call a function with an effect, your function either propagates it (declares the
same effect) or handles it (via `guard`/`catch`). The compiler enforces this — same as
how `throws` already works today, generalized to all effects.

### Under the Hood

All three compile to the same foundation:
- `throws` is a non-resumable effect (continuation discarded on failure)
- `yields` and `async` are resumable effects backed by LLVM coroutine intrinsics
  (`llvm.coro.begin`, `llvm.coro.suspend`, etc.)
- Both `yields` and `async` compile to state machines — same machinery, different
  poll interface

---

## What Effects Replace

| Before | After | What gets deleted |
|--------|-------|-------------------|
| `Result[T, E]` / `T throws E` via Tryable | `Fail[E]` effect | `Tryable`, `ControlFlow`, `FromResidual`, `FromValue` — four protocols |
| `Iterator` protocol + ~20 adapter structs | `yields T` generators | `MapIterator`, `FilterIterator`, `TakeIterator`, `SkipIterator`, `ZipIterator`, `ChainIterator`, `FlatMapIterator`, `EnumerateIterator`, `ResultIterator`... |
| No async (planned as separate feature) | `async` effect | Clean slate — no function coloring problem |

---

## Generators — `yields T`

### Writing Generators

A generator is just a function that yields values. No state machine struct, no
`next()` method, no protocol conformance:

```kestrel
func fibonacci() yields Int64 {
    var a: Int64 = 0
    var b: Int64 = 1
    loop {
        yield a
        let next = a + b
        a = b
        b = next
    }
}

func countdown(from n: Int64) yields Int64 {
    var i = n
    while i > 0 {
        yield i
        i -= 1
    }
}
```

### Consuming Generators

```kestrel
for value in countdown(from: 5) {
    print(value)
}

let first10 = fibonacci().take(count: 10).collect()
```

### Iterable Protocol (Simplified)

```kestrel
// before: needs associated iterator type
protocol Iterable {
    type Item
    type Iter: Iterator where Iter.Item == Item
    func iter() -> Iter
}

// after: just a method that yields
protocol Iterable {
    type Item
    func iter() yields Item
}

extend Array[T]: Iterable {
    func iter() yields T {
        var i: Int64 = 0
        while i < self.count() {
            yield self(i)
            i += 1
        }
    }
}
```

### Adapters as Generator Transforms

Every adapter that was a stateful struct becomes a simple function:

```kestrel
extend<T> where T: Iterable {
    func map[U](transform: (Item) -> U) yields U {
        for item in self { yield transform(item) }
    }

    func filter(predicate: (Item) -> Bool) yields Item {
        for item in self { if predicate(item) { yield item } }
    }

    func take(count n: Int64) yields Item {
        var i: Int64 = 0
        for item in self {
            if i >= n { return }
            yield item
            i += 1
        }
    }

    func flatMap[U](transform: (Item) -> U) yields U.Item where U: Iterable {
        for item in self {
            for inner in transform(item) { yield inner }
        }
    }
}
```

Usage unchanged from the caller's perspective:

```kestrel
fibonacci()
    .filter { it % 2 == 0 }
    .map { it * 3 }
    .take(count: 10)
    .collect()
```

---

## Async

### Async Functions

```kestrel
func fetchUser(id: Int64) -> User async throws AppError {
    let response = try await http.get("/users/\{id}")
    response.body
}
```

### Structured Concurrency

```kestrel
// concurrent execution — both run, both awaited
let (user, posts) = await (fetchUser(id: 1), fetchPosts(userId: 1))

// spawn for fire-and-forget
let fiber = spawn fetchUser(id: 1)
// ... do other work ...
let user = await fiber
```

### Combinators as Methods on Task

Timeout, retry, race, etc. are methods on the Task type, not effect handlers:

```kestrel
let user = fetchUser(id: 1)
    .timeout(seconds: 5)
    .retry(attempts: 3)
    .await
```

### Executor Model

Executor-agnostic with a built-in default. Async is Rust-style: stackless coroutines
compiled to state machines via LLVM coroutines.

```kestrel
// built-in: stackless state machines (Rust-like, low memory)
std.async.run(myProgram)

// alternative: fiber-based (Go-like, for blocking C interop)
std.async.runFibers(myProgram)

// test: deterministic, single-threaded
TestExecutor.run(myProgram)
```

The executor IS an effect handler for `Async`. Custom executors are custom handlers.
User code is identical regardless of executor choice.

### Async Main

```kestrel
func main() async {
    await doWork()
}
```

`main` declared `async` automatically uses the built-in executor.

---

## Error Handling — `throws`/`try` as Effect

### Throws Functions

```kestrel
func parse(s: String) -> Int64 throws ParseError {
    // ...
}

// call site unchanged
let n = try parse("42")
```

### Non-resumable by Default, Resumable When Needed

`try` discards the continuation on failure (same as `Result` semantics). But the
continuation exists as an algebraic effect, so `guard`/`catch` can resume:

```kestrel
// normal: non-resuming
let n = try parse(s)

// advanced: resume with a fallback value
let config = guard {
    try loadConfig(path: "app.conf")
} catch {
    error(e) => resume(Config.default())
}
```

---

## Effect Polymorphism

Functions can be generic over effects. This eliminates Rust's sync/async duplication
problem — one function handles all effect combinations.

```kestrel
// E is an effect variable — map propagates whatever effects the callback has
func map[T, U, with E](array: Array[T], transform: (T) -> U with E) -> Array[U] with E {
    var result: Array[U] = []
    for item in array {
        result.append(transform(item))
    }
    result
}
```

One function, every possible use:

```kestrel
// pure — E is empty
let doubled = map([1, 2, 3]) { it * 2 }

// throws — E is Fail[ParseError]
let parsed = try map(strings) { try Int64.parse(it) }

// async — E is Async
let users = await map(ids) { await fetchUser(id: it) }

// async + throws — E is Async + Fail[AppError]
let users = try await map(ids) { try await fetchUser(id: it) }
```

Extends to all higher-order functions:

```kestrel
func retry[T, with E](attempts n: Int64, body: () -> T with E) -> T with E {
    var last: Optional[Error] = .None
    var i: Int64 = 0
    while i < n {
        guard { return body() } catch {
            error(e) => { last = .Some(e); i += 1 }
        }
    }
    throw last.unwrap()
}

func withFile[T, with E](path: String, body: (File) -> T with E) -> T throws IOError with E {
    let file = try File.open(path)
    let result = body(file)
    file.close()
    result
}

// works sync or async — same function
let data = try withFile("data.txt") { it.readAll() }
let data = try await withFile("data.txt") { await it.readAllAsync() }
```

---

## Effects and Values

Effects and values are two views of the same thing. Effects are the ergonomic default
for control flow. Values are the escape hatch when you need to store, pass, or
manipulate computations as first-class data.

| Effect | Value form | Reify (effect → value) | Unreify (value → effect) |
|--------|-----------|----------------------|------------------------|
| `throws E` | `Result[T, E]` | `Result.from { try expr }` | `try result` |
| `async` | `Task[T]` | `Task { await expr }` | `await task` |
| `yields T` | `Generator[T]` | `Generator { yield expr }` | `for x in gen` |

### When to Use Values

Values are first-class — you can store them in arrays, pass them around, return them:

```kestrel
// collect async tasks into an array
let tasks: Array[Task[User]] = ids.map { Task { await fetchUser(id: it) } }
let results = await Task.all(tasks)

// store a Result without handling it
let result = Result.from { try parse(s) }

// value combinators for simple transforms
parse(s).map { it * 2 }.unwrapOr(default: 0)
```

### When to Use Effects

Effects give better ergonomics for sequential control flow:

```kestrel
// effects: linear, reads top-to-bottom
let user = try await fetchUser(id: 1)
let posts = try await fetchPosts(userId: user.id)
let rendered = try renderProfile(user, posts)

// values: same thing with nested flatMap
fetchUser(id: 1).flatMap { user in
    fetchPosts(userId: user.id).flatMap { posts in
        renderProfile(user, posts)
    }
}
```

Effects avoid nested wrapper types (`Future[Result[Optional[T], E]]`) and provide
effect polymorphism (one function works for pure, throwing, async).

`Result[T, E]` doesn't go away — it stays as the value form of `throws`. You just
rarely need it because `try`/`catch` handles the common case.

---

## `given` — Ambient Scoped Values

`given` provides a value in scope. Functions that use it don't need to declare
anything — it's ambient, invisible in signatures, and unloads at end of scope.

### Orthogonality with Effects

- **Effects** change how a function behaves (`throws` can short-circuit, `async`
  can suspend) — they propagate in type signatures
- **`given`** changes what a function has access to — invisible, ambient, doesn't
  propagate

### Basic Usage

```kestrel
given Allocator = ArenaAllocator(capacity: 4 * 1024 * 1024)
given Random = SystemRandom()

buildIndex(words: corpus)   // uses ArenaAllocator internally — no annotation
shuffle(items: deck)        // uses SystemRandom internally — transparent
// both freed at end of scope
```

### Nested Scoping (Shadowing)

Inner `given` shadows outer for the nested scope:

```kestrel
func processRequest(req: Request) -> Response {
    given Allocator = ArenaAllocator(capacity: 4 * 1024 * 1024)
    let parsed = parseRequest(req)   // arena

    {
        given Allocator = ScratchAllocator(capacity: 64 * 1024)
        let tokens = tokenize(parsed)    // scratch
        fillResponse(&response, tokens)
    }  // scratch freed, tokens gone

    response  // safely on arena
}
```

### Allocator in Generic Params

Types can carry their allocator for lifetime safety:

```kestrel
struct Array[T, A: Allocator = GlobalAllocator] { ... }

// type system catches escaping references
func bad() -> Array[Int64] {
    given Allocator = ScratchAllocator()
    let arr = Array[Int64].new()  // Array[Int64, ScratchAllocator]
    arr  // compile error — ScratchAllocator doesn't outlive this scope
}
```

### Accessing the Given Value

```kestrel
func sample(records: Array[Record], count n: Int64) -> Array[Record] {
    var pool = records.clone()
    var result: Array[Record] = []
    var i: Int64 = 0
    while i < n && !pool.isEmpty() {
        let idx = (given Random).next(in: 0..<pool.count())
        result.append(pool.remove(at: idx))
        i += 1
    }
    result
}
```

### Standard Given Values

| Given | Default | Use case |
|-------|---------|----------|
| `Allocator` | `GlobalAllocator` (heap) | Memory allocation strategy |
| `Random` | `SystemRandom()` | Testable randomness |

`given` is NOT used for IO (same "it's everywhere" problem as an IO effect).

---

## `guard`/`catch` — Effect Handling

### Block Form

`guard` wraps the protected code. `catch` provides pattern-matching handlers:

```kestrel
guard {
    let u = try user.get()
    let p = try posts.get()
    ProfileView(user: u, posts: posts)
} catch {
    loading => Skeleton(),
    error(e) => ErrorView(e)
}
```

### Inline Form

`.catch` on any expression for quick fallbacks:

```kestrel
// single fallback
let name = try user.get().name.catch { "Anonymous" }

// specific effect
UserProfile(id: 1)
    .catch(loading) { Skeleton() }
    .catch(e: AppError) { ErrorView(e) }
```

### Resumable Catch

Provide a fallback value and continue the computation:

```kestrel
let config = guard {
    try loadConfig(path: "app.conf")
} catch {
    error(e) => resume(Config.default())
}
```

### Nested Handlers

Inner catches handle first, unhandled effects propagate up:

```kestrel
guard {
    guard {
        UserProfile(id: 1)
    } catch {
        loading => Skeleton()  // handles loading
    }
    // errors NOT caught here — propagate to outer
} catch {
    error(e) => FullScreenError(e)  // catches errors from anywhere inside
}
```

### Syntax Summary

| Form | Use case |
|------|----------|
| `expr.catch { fallback }` | Inline, quick fallback |
| `expr.catch(loading) { fallback }` | Inline, specific effect |
| `guard { body } catch { arms }` | Block, multiple effects, pattern matching |
| `guard { body } catch { resume(val) }` | Resumable — continue with fallback value |

Both forms desugar to effect handlers.

---

## UI Component Model

Components are `@reactive` structs. No hooks, no property wrappers, no dependency
arrays. Effects enable components to write only the happy path — ancestors handle
loading and errors.

### Component Basics

```kestrel
@reactive
struct Counter: Component {
    var count = 0

    func body() -> View {
        Button("Count: \{count}") { count += 1 }
    }
}
```

- `@reactive` attribute desugars `var` → reactive cells, `let` → computed values
- State is `var`, derived state is `let`, props are `let` fields
- `body()` returns a View — the reactive system handles re-evaluation on state change

### Props and Bindings

```kestrel
@reactive
struct SearchBar: Component {
    var text: String          // mutable — can receive &binding from parent
    let placeholder: String   // immutable — regular prop

    func body() -> View {
        TextField(text: &text, placeholder: placeholder)
    }
}

@reactive
struct Parent: Component {
    var query = ""

    func body() -> View {
        Column {
            SearchBar(text: &query, placeholder: "Search...")
            Text("Searching: \{query}")
        }
    }
}
```

No `@Binding`. No `@State`. Just `var` (mutable, bindable) vs `let` (immutable, prop).

### Data Fetching

`query` and `mutation` are field modifiers:

```kestrel
@reactive
struct UserProfile: Component {
    let id: Int64
    query user = { await fetchUser(id: id) }

    mutation updateName = { name: String in
        user.optimistic { u in u.withName(name) }
        match await patchUser(id: id, name: name) {
            .Ok(_) => (),
            .Err(_) => user.invalidate()
        }
    }

    func body() -> View throws async {
        let u = try user.get()
        Column {
            Avatar(url: u.avatar)
            Text(u.name)
        }
    }
}
```

### Reactive Props

Parameters are reactive bindings, not fixed values. When the parent re-evaluates and
passes new props, the component updates without restarting — local state survives:

```kestrel
@reactive
struct ChatRoom: Component {
    let roomId: String   // reactive binding from parent
    query messages = { await fetchMessages(roomId: roomId) }
    var draft = ""       // survives roomId changes

    func body() -> View {
        Column {
            match messages {
                .Loading => MessagesSkeleton(),
                .Data(let msgs) => List(msgs, key: { it.id }) { _, m in
                    MessageBubble(m)
                }
            }
            TextField(text: &draft)
        }
    }
}
```

### Effects in Components — Happy Path Only

Component bodies can `throw` and be `async`. Ancestors handle with `guard`/`catch`:

```kestrel
@reactive
struct ProfilePage: Component {
    let id: Int64
    query user = { await fetchUser(id: id) }

    func body() -> View throws async {
        let u = try user.get()
        Column { Avatar(url: u.avatar); Text(u.name) }
    }
}

@reactive
struct App: Component {
    func body() -> View {
        guard {
            ProfilePage(id: 1)
        } catch {
            loading => PageSkeleton(),
            error(e) => ErrorBanner(e)
        }
    }
}
```

Components only write the success case. Loading and error handling are pushed to the
boundary — consistent treatment, one place to change.

### Lifecycle via Structured Concurrency

```kestrel
@reactive
struct LiveTimer: Component {
    var elapsed: Int64 = 0

    task {
        loop { await sleep(Duration.seconds(1)); elapsed += 1 }
    }

    func body() -> View {
        Text("Elapsed: \{elapsed}s")
    }
}
```

| Event | What happens |
|-------|-------------|
| Mount | Component created, `task` blocks start |
| Update | State mutates → `body()` re-evaluates → runtime diffs → patches UI |
| Unmount | Component dropped → structured concurrency cancels all tasks |

No `onAppear`/`onDisappear`/`useEffect`. Cancellation propagates automatically.

### Context via `given`

```kestrel
@reactive
struct App: Component {
    var darkMode = false

    given Theme = if darkMode { dark } else { light }
    given Router = Router.new(initial: "/")

    func body() -> View {
        NavStack(background: (given Theme).background) {
            guard {
                match (given Router).route {
                    "/" => TodoList(),
                    "/user/:id" => UserProfile(id: (given Router).param("id"))
                }
            } catch {
                loading => PageSkeleton(),
                error(e) => ErrorBanner(e)
            }
        }
    }
}
```

### What Replaces What

| React | SwiftUI | Kestrel |
|-------|---------|---------|
| `useState(0)` | `@State var x = 0` | `var x = 0` |
| `useMemo(() => ..., [deps])` | computed property | `let x = ...` |
| `useCallback(fn, [deps])` | N/A | closures are stable |
| `useRef(x)` | plain var | `var x = ...` |
| `useEffect(() => ..., [deps])` | `.task { }` | `task { }` |
| `useContext(Ctx)` | `@Environment(\.key)` | `given Theme` |
| `<Suspense>` + `<ErrorBoundary>` | doesn't exist | `guard { } catch { }` |
| React Query `useQuery` | doesn't exist | `query` field |
| React Query `useMutation` | doesn't exist | `mutation` field |

---

## Compiler vs. Library

### Compiler — Language Features

| Feature | Description |
|---------|-------------|
| Effect rows | New position in function types, inference propagates effects |
| `yield` keyword | Compiles to `llvm.coro.suspend` state machine |
| `await` keyword | Same LLVM coroutine machinery, different poll interface |
| `throws`/`try` as effect | Rewrite of existing feature, delete Tryable machinery |
| `given` scoping | `given T = expr` introduces binding, `(given T)` resolves scope chain |
| `guard`/`catch` blocks | Match-like syntax over effect operations |
| `@reactive` attribute | Desugars `var` → `State[T]`, `let` → `Computed[T]` |

### Stdlib — Library Code

| Module | Contents |
|--------|----------|
| `std.async` | Built-in executor, `Task` type, `spawn`, `sleep` |
| `std.async.fiber` | Fiber-based executor (alternative handler) |
| `std.random` | `SystemRandom`, `DeterministicRandom` |
| `std.memory` | `Allocator` protocol (already exists) |

### UI Framework — Library, Could Be Separate Package

| Module | Contents |
|--------|----------|
| `ui.core` | `View`, `Column`, `Row`, `Text`, `Button`, `TextField`, `List` |
| `ui.state` | `State[T]`, `Computed[T]` reactive primitives |
| `ui.data` | `query()`, `mutation()`, `QueryCache` |
| `ui.runtime` | Reconciler, renderer, event dispatch |

---

## Implementation Phases

| Phase | Work | Depends on |
|-------|------|-----------|
| 1 | Effect rows in type system | — |
| 2 | `throws`/`try` as effect — delete `Tryable`/`ControlFlow`/`FromResidual`/`FromValue` | Phase 1 |
| 3 | `given` scoping | — |
| 4 | `guard`/`catch` blocks | Phase 1 |
| 5 | `yield` + LLVM coroutines → generators work | Phase 1 |
| 6 | Replace `Iterator` protocol with generators | Phase 5 |
| 7 | `await` + async effect | Phase 5 |
| 8 | `std.async` — executor, Task, spawn | Phase 7 |
| 9 | `@reactive` attribute | Phase 3 |
| 10 | UI framework (library) | Phase 8, 9 |

Phases 1-5 are the core compiler work. Everything after builds on those primitives
as library code.
