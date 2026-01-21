# Kestrel Language Design

## Core Principles

### 1. Protocols Are the Universal Abstraction

Everything is a protocol. Operators, iterators, futures, memory semantics — all protocols. If you can define it as a protocol, it belongs in the library, not the language.

### 2. Defaults with Escape Hatches

Every decision has a sensible default and an explicit override:

| Concern | Default | Override |
|---------|---------|----------|
| Mutability | `let` | `var` |
| Visibility | `internal` | `public`, `private`, `fileprivate` |
| Struct copy | Copy | `NotCopyable` protocol |
| Struct drop | Drop | `NotDroppable` protocol |
| Memory | Struct (value) | `class` (RC) |

Make the right thing easy, the wrong thing possible.

### 3. Syntax Reveals Semantics

If it changes control flow or has runtime cost, it has syntax:

```kestrel
try      // This can fail
await    // This suspends for async
yield    // This suspends for generator
?.       // This might be nil
```

No hidden behavior behind innocent-looking calls.

### 4. Return Type Drives Suspension

`await` and `yield` aren't effects that propagate — they're operations that talk to your return type. The `async` and `generator` flags enable suspension and determine the return type:

```kestrel
async func foo() -> T              // Returns some Future[T]
generator func foo() -> T          // Returns some Generator[T]
async generator func foo() -> T    // Returns some AsyncGenerator[T]
```

No colored functions. No effect systems. Just flags and types.

### 5. Composition Over Inheritance

No class inheritance. Ever.

- Structs for data (value types)
- Classes for shared state (reference counted)
- Protocols for abstraction
- Extensions for retroactive conformance

### 6. Library Over Language

The language provides minimal primitives. Everything else is library:

| Language | Library |
|----------|---------|
| `async`, `await`, `yield` keywords | `Future`, `Generator`, `AsyncGenerator` protocols |
| State machine transform | Executor implementations |
| `?.` operator | `Optional` type |
| `for` loops | `Iterable` protocol |
| `try`/`throws` | `Result` type |
| Copy/drop behavior | `NotCopyable`, `NotDroppable` protocols |

Swappable runtimes, custom suspendables, user-defined behavior — all possible because the language doesn't hardcode behavior.

### 7. Bare Metal to Full-Featured

The language scales from bare metal to application code. Features unlock as capabilities become available:

| Level | You Have | You Can Use |
|-------|----------|-------------|
| **0 - Bare Metal** | Nothing | Structs, enums, functions, generics, protocols, control flow |
| **1 - Allocator** | Allocator | Heap allocation, `String`, `Array`, `class` |
| **2 - Async Runtime** | Executor | `async`/`await`, `Future` |
| **3 - Full Stdlib** | All | Everything |

---

## Features

### Enums and Pattern Matching

**Status: ✓ IMPLEMENTED**

Algebraic data types with associated values and exhaustive pattern matching.

#### Enum Declaration

```kestrel
enum Option[T] {
    Some(T)
    None
}

enum Result[T, E] {
    Ok(T)
    Err(E)
}

enum Expr {
    Literal(Int)
    Binary(left: Expr, op: String, right: Expr)
    Unary(op: String, expr: Expr)
}
```

#### Pattern Matching

```kestrel
func unwrap[T](opt: Option[T], default: T) -> T {
    match opt {
        Some(value) => value
        None => default
    }
}

func eval(expr: Expr) -> Int {
    match expr {
        Literal(n) => n
        Binary(left, "+", right) => eval(left) + eval(right)
        Binary(left, "-", right) => eval(left) - eval(right)
        Unary("-", inner) => -eval(inner)
        _ => 0
    }
}
```

#### Guard Clauses

```kestrel
func describe(n: Int) -> String {
    match n {
        x if x < 0 => "negative"
        0 => "zero"
        x if x > 100 => "large"
        x => "small positive"
    }
}
```

#### If Let / Guard Let

```kestrel
func process(opt: Option[User]) {
    if let Some(user) = opt {
        print(user.name)
    }
}

func require(opt: Option[User]) -> User {
    guard let Some(user) = opt else {
        return defaultUser
    }
    user
}
```

---

### Type Inference

**Status: ✓ IMPLEMENTED**

Local type inference so explicit annotations are optional when types can be inferred.

```kestrel
let x = 42                    // Inferred as Int
let name = "Alice"            // Inferred as String
let point = Point(x: 1, y: 2) // Inferred as Point

let users = fetchUsers()      // Inferred from return type
let doubled = x * 2           // Inferred as Int
```

Generic type argument inference:

```kestrel
func identity[T](value: T) -> T { value }

let n = identity(42)          // Inferred as identity[Int](42)
let s = identity("hello")     // Inferred as identity[String]("hello")
```

---

### Closures

**Status: ✓ IMPLEMENTED**

First-class functions with captured variables.

#### Syntax

```kestrel
// Full syntax
let add = { (a: Int, b: Int) -> Int in a + b }

// Inferred parameter types
let add: (Int, Int) -> Int = { a, b in a + b }

// Single expression, inferred return
let double = { x in x * 2 }

// Shorthand arguments
let double: (Int) -> Int = { $0 * 2 }
```

#### Trailing Closures

```kestrel
list.map { $0 * 2 }

list.filter { $0 > 10 }
    .map { $0.toString() }
    .forEach { print($0) }

button.onClick {
    submit()
}
```

#### Capturing

```kestrel
func makeCounter() -> () -> Int {
    var count = 0
    return {
        count = count + 1
        count
    }
}

let counter = makeCounter()
counter()  // 1
counter()  // 2
```

---

### Memory Model

**Status: ✓ IMPLEMENTED** (Note: NotDroppable is not implemented)

Value types by default with reference counting for shared ownership and protocols for copy/drop behavior.

#### Structs (Value Types)

```kestrel
struct Point {
    var x: Int
    var y: Int
}

let p1 = Point(x: 1, y: 2)
let p2 = p1              // Copy
print(p1.x)              // Last use — optimized to move
```

#### Classes (Reference Counted)

```kestrel
class User {
    var name: String
}

let u1 = User(name: "Alice")
let u2 = u1              // RC increment
// RC decremented when out of scope
```

#### NotCopyable Protocol

```kestrel
struct Buffer: NotCopyable {
    var data: [UInt8]
}

let b1 = Buffer(data: [1, 2, 3])
// let b2 = b1          // Error: Buffer is not copyable
let b2 = move b1        // OK: explicit move
// b1 no longer valid
// b2 can be dropped silently
```

#### NotDroppable Protocol

```kestrel
struct Token: NotDroppable {
    var id: Int

    consuming func redeem() { ... }
}

let t = Token(id: 1)
// t goes out of scope   // Error: must consume
t.redeem()               // OK: consumed
```

#### Linear Types (Sugar)

```kestrel
// linear = NotCopyable + NotDroppable
linear struct FileHandle {
    private var fd: Int

    static func open(path: String) -> FileHandle throws { ... }

    consuming func close() { ... }

    func read() -> [UInt8] { ... }
}

func process() {
    let file = try FileHandle.open("data.txt")
    let data = file.read()
    file.close()         // Must consume
}
```

#### Memory Model Summary

| Declaration | Copy | Drop |
|-------------|------|------|
| `struct Foo` | Yes | Yes |
| `struct Foo: NotCopyable` | No | Yes |
| `struct Foo: NotDroppable` | Yes | No |
| `struct Foo: NotCopyable, NotDroppable` | No | No |
| `linear struct Foo` | No | No |
| `class Foo` | RC | Yes |

#### Reference Modifiers

```kestrel
struct Point {
    var x: Int
    var y: Int

    func length() -> Float { ... }           // Borrows self (default)

    mutating func translate(dx: Int, dy: Int) {
        x = x + dx
        y = y + dy
    }

    consuming func intoTuple() -> (Int, Int) {
        (x, y)
    }
}
```

---

### Error Handling

**Status: ⚠ PARTIAL** (Result/try implemented, catch not implemented)

Result types with syntactic sugar.

#### Result Type

```kestrel
enum Result[T, E] {
    Ok(T)
    Err(E)
}
```

#### Throws Sugar

```kestrel
// These are equivalent:
func parse(s: String) -> Ast throws ParseError { ... }
func parse(s: String) -> Result[Ast, ParseError] { ... }
```

#### Try Sugar

```kestrel
func compile(source: String) -> Binary throws CompileError {
    let ast = try parse(source)
    let ir = try analyze(ast)
    let binary = try codegen(ir)
    binary
}
```

#### Try With Catch

```kestrel
func handleErrors(source: String) {
    try {
        let binary = try compile(source)
        run(binary)
    } catch ParseError(msg) {
        print("Parse error: \(msg)")
    } catch AnalyzeError(msg) {
        print("Analysis error: \(msg)")
    } catch e {
        print("Unknown error: \(e)")
    }
}
```

---

### Optional Chaining and Nil Coalescing

**Status: ✗ NOT IMPLEMENTED**

Safe navigation for optionals.

```kestrel
// Optional chaining
user?.profile?.settings?.theme

// Nil coalescing
let name = user?.name ?? "Anonymous"

// Combined
let theme = user?.profile?.settings?.theme ?? Theme.default
```

---

### Pipeline Operator

**Status: ✗ NOT IMPLEMENTED**

Left-to-right function composition.

```kestrel
// Instead of nested calls
let result = process(transform(parse(input)))

// Pipeline
let result = input |> parse |> transform |> process

// With closures
users
    |> filter { $0.active }
    |> map { $0.name }
    |> sorted
    |> join(", ")
```

---

### Ranges

**Status: ✗ NOT IMPLEMENTED**

Range syntax using `to` (inclusive) and `until` (exclusive).

```kestrel
0 until 10     // [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
0 to 10        // [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

for i in 0 until 100 {
    print(i)
}
```

---

### For Expressions as Iterators

**Status: ✗ NOT IMPLEMENTED**

For loops are expressions that return lazy iterators.

```kestrel
// As a loop
for i in 0 until 100 {
    print(i)
}

// As an expression (returns Generator)
let evens = for i in 0 until 100 if i % 2 == 0 yield i

// With transformation
let squares = for i in 0 until 10 yield i * i

// Multiple generators
let pairs = for x in 0 until 5, y in 0 until 5 if x != y yield (x, y)

// Eager collection
let list = (for i in 0 until 10 yield i * 2).collect()
```

---

### Single Expression Function Bodies

**Status: ✗ NOT IMPLEMENTED**

Concise syntax for simple functions.

```kestrel
func double(x: Int) -> Int = x * 2

func greet(name: String) -> String = "Hello, \(name)"

func isEven(n: Int) -> Bool = n % 2 == 0
```

---

### Implicits (Scala 3 Style)

**Status: ✗ NOT IMPLEMENTED**

Compile-time dependency injection.

#### Declaration

```kestrel
protocol Database {
    func query(sql: String) -> Rows throws
    func execute(sql: String) throws
}

protocol Logger {
    func info(message: String)
    func error(message: String)
}
```

#### Using Parameters

```kestrel
func createUser(name: String)(using db: Database, logger: Logger) -> User throws {
    logger.info("Creating user: \(name)")
    let user = User(name: name)
    try db.execute("INSERT INTO users ...")
    user
}
```

#### Given Instances

```kestrel
given Database = PostgresDatabase(connectionString: "...")
given Logger = ConsoleLogger()

// Call without explicit arguments
let user = try createUser(name: "Alice")
```

#### Scoped Rebinding

```kestrel
func test() {
    given Database = MockDatabase()
    given Logger = NullLogger()

    let user = try createUser(name: "Test")
    assert(user.name == "Test")
}
```

---

### Suspendable Functions

**Status: ✗ NOT IMPLEMENTED**

Functions can be flagged with `async` and/or `generator` to enable suspension. The compiler generates state machines conforming to the appropriate protocol.

#### Protocols

```kestrel
protocol Yieldable[T] {
    mutating func next() -> Option[T]
}

protocol Awaitable[T] {
    func poll(waker: Waker) -> Poll[T]
}

protocol Generator[T]: Yieldable[T], Iterable[T] {}

protocol Future[T]: Awaitable[T] {}

protocol AsyncGenerator[T]: Yieldable[T] {
    mutating func next() -> Future[Option[T]]
}

enum Poll[T] {
    Ready(T)
    Pending
}
```

#### Function Flags

| Flag | Enables | Return Type |
|------|---------|-------------|
| `async` | `await` | `some Future[T]` |
| `generator` | `yield` | `some Generator[T]` |
| `async generator` | both | `some AsyncGenerator[T]` |

#### Custom Suspendable Types

```kestrel
protocol CancellableFuture[T]: Awaitable[T] {
    func cancel()
}

async(CancellableFuture) func longOperation() -> Result {
    await step1()
    await step2()
}
// Returns: some CancellableFuture[Result]

protocol Stream[T]: Yieldable[T] { ... }

generator(Stream) func events() -> Event {
    loop {
        yield nextEvent()
    }
}
// Returns: some Stream[Event]
```

---

### Generators

Lazy sequences via the `generator` flag.

```kestrel
generator func range(start: Int, end: Int) -> Int {
    var i = start
    while i < end {
        yield i
        i = i + 1
    }
}

generator func fibonacci() -> Int {
    var (a, b) = (0, 1)
    loop {
        yield a
        (a, b) = (b, a + b)
    }
}

// Usage
for n in fibonacci().take(10) {
    print(n)
}
```

#### Compiler Transformation

```kestrel
// You write:
generator func countdown(n: Int) -> Int {
    var i = n
    while i >= 0 {
        yield i
        i = i - 1
    }
}

// Compiler generates state machine:
struct __countdown: Generator[Int] {
    var n: Int
    var i: Int
    var state: Int = 0

    mutating func next() -> Option[Int] {
        loop {
            switch state {
            case 0:
                i = n
                state = 1
                continue
            case 1:
                if i >= 0 {
                    state = 2
                    return .Some(i)
                } else {
                    return .None
                }
            case 2:
                i = i - 1
                state = 1
                continue
            }
        }
    }
}
```

---

### Async/Await

Asynchronous programming via the `async` flag.

```kestrel
async func fetchUser(id: Int) -> User {
    let response = await http.get("/users/\(id)")
    await response.json()
}

async func fetchAll(ids: [Int]) -> [User] {
    let tasks = ids.map { id in spawn fetchUser(id) }
    tasks.map { task in await task }
}
```

#### Compiler Transformation

```kestrel
// You write:
async func fetchBoth() -> (User, Posts) {
    let user = await fetchUser(1)
    let posts = await fetchPosts(user.id)
    (user, posts)
}

// Compiler generates state machine:
struct __fetchBoth: Future[(User, Posts)] {
    var user: Option[User] = .None
    var posts: Option[Posts] = .None
    var state: Int = 0
    var pending: Option[Any] = .None

    func poll(waker: Waker) -> Poll[(User, Posts)] {
        loop {
            switch state {
            case 0:
                pending = .Some(fetchUser(1))
                state = 1
                continue
            case 1:
                switch (pending as Future[User]).poll(waker) {
                case .Pending: return .Pending
                case .Ready(let u):
                    user = .Some(u)
                    state = 2
                    continue
                }
            case 2:
                pending = .Some(fetchPosts(user!.id))
                state = 3
                continue
            case 3:
                switch (pending as Future[Posts]).poll(waker) {
                case .Pending: return .Pending
                case .Ready(let p):
                    return .Ready((user!, p))
                }
            }
        }
    }
}
```

---

### Async Generators

Combining both flags for async iteration.

```kestrel
async generator func livePrices(symbol: String) -> Price {
    let socket = await connect(symbol)
    loop {
        let price = await socket.receive()
        yield price
    }
}

async generator func fetchPages(url: String) -> Page {
    var nextUrl: Option[String] = .Some(url)
    while let Some(u) = nextUrl {
        let page = await fetch(u)
        yield page
        nextUrl = page.nextPageUrl
    }
}

// Usage
async for price in livePrices("AAPL") {
    print(price)
}
```

---

### Swappable Runtimes

Async execution is driven by library-provided runtimes, injected via implicits.

#### Executor Protocol

```kestrel
protocol Executor {
    func spawn[T](future: Future[T]) -> Task[T]
    func block[T](future: Future[T]) -> T
}
```

#### Runtime Implementations

```kestrel
// Simple single-threaded
struct SimpleExecutor: Executor { ... }

// Work-stealing multi-threaded
struct WorkStealingExecutor: Executor {
    var threadCount: Int
    ...
}
```

#### Selection via Implicits

```kestrel
func main() {
    given Executor = WorkStealingExecutor(threadCount: 4)

    Executor.block {
        let users = await fetchAll([1, 2, 3])
        print(users)
    }
}

func test() {
    given Executor = SimpleExecutor()

    Executor.block {
        await testAsync()
    }
}
```

---

### Unsafe

Escape hatch for bare metal programming.

```kestrel
unsafe func writeRegister(addr: UInt32, value: UInt32) {
    let ptr = UnsafePointer[UInt32](addr)
    ptr.write(value)
}

unsafe func readVolatile[T](addr: UInt) -> T {
    let ptr = UnsafePointer[T](addr)
    ptr.readVolatile()
}

// Inline assembly (future)
unsafe func halt() {
    asm("hlt")
}
```

---

### String Interpolation

**Status: ✓ IMPLEMENTED**

Swift-style string interpolation.

```kestrel
let name = "Alice"
let age = 30

let greeting = "Hello, \(name)!"
let info = "\(name) is \(age) years old"
let computed = "Double: \(age * 2)"
```

---

### Extensions

**Status: ✓ IMPLEMENTED**

Add methods and protocol conformances to existing types.

#### Adding Methods

```kestrel
extend String {
    func reversed() -> String {
        // implementation
    }

    var lines: [String] {
        self.split("\n")
    }
}
```

#### Retroactive Conformance

```kestrel
extend Int: Monoid {
    static var empty: Int = 0

    func combine(other: Int) -> Int {
        self + other
    }
}
```

#### Conditional Conformance

```kestrel
extend Array[T]: Equatable where T: Equatable {
    func equals(other: Array[T]) -> Bool {
        // implementation
    }
}
```
