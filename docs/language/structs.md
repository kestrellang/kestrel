# Structs

Structs are value types that encapsulate state and behavior. They are the primary way to define custom data types in Kestrel.

## Declaration Syntax

### Basic Struct

```kestrel
struct Point {
    var x: Int
    var y: Int
}
```

### Empty Struct

```kestrel
struct Empty {}
```

### Visibility Modifiers

Structs can use any of the five visibility levels:

```kestrel
public struct PublicStruct {}
private struct PrivateStruct {}
internal struct InternalStruct {}
fileprivate struct FileprivateStruct {}
struct DefaultStruct {}  // Defaults to internal
```

### Nested Structs

Structs can be nested inside other structs:

```kestrel
struct Outer {
    var field: Int

    struct Inner {
        var value: Int
    }
}

// Access: Outer.Inner
```

## Fields (Stored Properties)

Fields store data in the struct. They can be mutable (`var`) or immutable (`let`).

### Mutable Fields

```kestrel
struct Counter {
    var count: Int
}
```

### Immutable Fields

```kestrel
struct Point {
    let x: Int
    let y: Int
}
```

### Mixed Mutability

```kestrel
struct User {
    let id: Int       // Immutable - cannot change after initialization
    var name: String  // Mutable - can be changed
}
```

### Field Visibility

Fields can have visibility modifiers:

```kestrel
struct Person {
    public var name: String
    private var ssn: String
    internal var age: Int
}
```

## Computed Properties

Computed properties provide derived values without storing them. See [Computed Properties](computed-properties.md) for full details.

### Getter-Only (Shorthand)

```kestrel
struct Rectangle {
    var width: Float
    var height: Float

    var area: Float { self.width * self.height }
    var perimeter: Float { 2 * (self.width + self.height) }
}
```

### Getter and Setter

```kestrel
struct Temperature {
    private var kelvin: Float

    var celsius: Float {
        get { self.kelvin - 273.15 }
        set { self.kelvin = newValue + 273.15 }
    }
}
```

### Static Computed Properties

```kestrel
struct Int64 {
    public static var zero: Int64 { Int64(value: 0) }
    public static var max: Int64 { Int64(value: 9223372036854775807) }
}
```

## Initializers

Initializers create instances of a struct. There are two kinds: implicit memberwise initializers and explicit custom initializers.

### Implicit Memberwise Initializer

If no explicit `init` is defined, Kestrel provides an automatic memberwise initializer:

```kestrel
struct Point {
    var x: Int
    var y: Int
}

func makePoint() -> Point {
    Point(x: 10, y: 20)  // Memberwise initializer
}
```

### Explicit Initializers

Custom initializers use the `init` keyword. **Note**: Unlike Rust, initializers do not declare `self` as a parameter.

```kestrel
struct Point {
    var x: Int
    var y: Int

    init(x: Int, y: Int) {
        self.x = x;
        self.y = y;
    }
}
```

### Initializer with No Parameters

```kestrel
struct Counter {
    var count: Int

    init() {
        self.count = 0;
    }
}
```

### Multiple Initializers

Structs can have multiple initializers with different signatures (overloading):

```kestrel
struct Point {
    var x: Int
    var y: Int

    init(x: Int, y: Int) {
        self.x = x;
        self.y = y;
    }

    init() {
        self.x = 0;
        self.y = 0;
    }

    init(value: Int) {
        self.x = value;
        self.y = value;
    }
}

func test() {
    let p1 = Point(x: 1, y: 2);
    let p2 = Point();
    let p3 = Point(value: 5);
}
```

### Initializer with Labeled Parameters

Parameters can have external labels (argument labels) and internal names:

```kestrel
struct Point {
    var x: Int
    var y: Int

    init(atX x: Int, atY y: Int) {
        self.x = x;
        self.y = y;
    }
}

func makePoint() -> Point {
    Point(atX: 5, atY: 10)  // Use external labels
}
```

### Initializer Visibility

Initializers can have their own visibility:

```kestrel
public struct Point {
    var x: Int

    public init(x: Int) {
        self.x = x;
    }
}
```

## Deinitializers and RAII

Deinitializers clean up resources when a struct goes out of scope. This follows the RAII (Resource Acquisition Is Initialization) pattern.

### Deinit Block

```kestrel
struct File: not Copyable {
    var fd: Int

    deinit {
        if self.fd >= 0 {
            close(self.fd);
        }
    }
}
```

### When Deinit Runs

The `deinit` block runs automatically when:
- A local variable goes out of scope
- A struct field is replaced or the containing struct is destroyed
- An explicit `deinit` statement is used (for non-copyable types)

```kestrel
func example() {
    let file = File.open("data.txt");
    // Use file...
}  // deinit runs here automatically
```

### Explicit Deinit (Non-Copyable Types)

For non-copyable types, you can explicitly destroy a value:

```kestrel
func earlyCleanup() {
    var file = File.open("data.txt");
    // Use file...
    deinit file;  // Explicitly destroy early
    // file is no longer valid
}
```

## Instance Methods

Methods are functions defined inside a struct. **Unlike Rust, methods do not declare `self` as an explicit parameter.**

### Basic Methods

```kestrel
struct Rectangle {
    var width: Float
    var height: Float

    func area() -> Float {
        self.width * self.height
    }

    func perimeter() -> Float {
        2 * (self.width + self.height)
    }
}

func example() {
    let rect = Rectangle(width: 10.0, height: 5.0);
    let a = rect.area();      // 50.0
    let p = rect.perimeter(); // 30.0
}
```

### Mutating Methods

Methods that modify `self` must be marked with `mutating`:

```kestrel
struct Counter {
    var count: Int

    mutating func increment() {
        self.count = self.count + 1;
    }

    mutating func reset() {
        self.count = 0;
    }
}

func example() {
    var counter = Counter(count: 0);
    counter.increment();  // count is now 1
    counter.increment();  // count is now 2
    counter.reset();      // count is now 0
}
```

### Consuming Methods

Methods that consume `self` are marked with `consuming`. These take ownership of the value:

```kestrel
struct Resource: not Copyable {
    var handle: Int

    consuming func close() {
        // Resource cleanup
        // self is consumed and cannot be used after this
    }
}
```

### Method Modifiers Summary

| Modifier | Access to `self` | Use Case |
|----------|-----------------|----------|
| None | Read-only (borrowed) | Query operations, getters |
| `mutating` | Read-write | Modify struct state |
| `consuming` | Consumes value | Clean up, transform into different type |

## Static Methods and Properties

Static members belong to the type itself, not to instances.

### Static Methods

```kestrel
struct Point {
    var x: Int
    var y: Int

    static func origin() -> Point {
        Point(x: 0, y: 0)
    }

    static func fromValue(value: Int) -> Point {
        Point(x: value, y: value)
    }
}

func example() {
    let p1 = Point.origin();
    let p2 = Point.fromValue(5);
}
```

### Static Stored Properties

```kestrel
struct Config {
    static var version: Int = 1
    static var debugMode: Bool = false
}
```

### Static Computed Properties

```kestrel
struct Math {
    static var pi: Float { 3.14159265359 }
    static var e: Float { 2.71828182846 }
}
```

## Protocol Conformance

Structs can conform to protocols by implementing their required methods and properties.

```kestrel
protocol Drawable {
    func draw() -> String
}

struct Circle: Drawable {
    var radius: Float

    func draw() -> String {
        "Circle with radius: " + toString(self.radius)
    }
}

struct Rectangle: Drawable {
    var width: Float
    var height: Float

    func draw() -> String {
        "Rectangle: " + toString(self.width) + "x" + toString(self.height)
    }
}
```

### Multiple Protocol Conformance

```kestrel
protocol Named {
    var name: String { get }
}

protocol Identified {
    var id: Int { get }
}

struct User: Named, Identified {
    var name: String
    var id: Int
}
```

## Generic Structs

Structs can be generic over one or more type parameters.

### Single Type Parameter

```kestrel
struct Box[T] {
    var value: T

    init(value: T) {
        self.value = value;
    }

    func getValue() -> T {
        self.value
    }
}

func example() {
    let intBox = Box[Int](value: 42);
    let stringBox = Box[String](value: "hello");
}
```

### Multiple Type Parameters

```kestrel
struct Pair[A, B] {
    var first: A
    var second: B

    init(first: A, second: B) {
        self.first = first;
        self.second = second;
    }
}

func example() {
    let pair = Pair[Int, String](first: 1, second: "one");
}
```

### Generic Methods

```kestrel
struct Container[T] {
    var items: Array[T]

    func map[U](transform: (T) -> U) -> Container[U] {
        // Transform each item
    }
}
```

### Where Clauses

Generic structs can constrain their type parameters:

```kestrel
struct SortedArray[T] where T: Comparable {
    var items: Array[T]

    func insert(item: T) {
        // Insert maintaining sorted order
    }
}
```

### Non-Copyable Generic Types

By default, generic types assume `Copyable`. To support non-copyable types:

```kestrel
struct Container[T] where T: not Copyable {
    var value: T
}
```

## Copy Semantics

**Structs are copy-by-default in Kestrel.** This is a fundamental difference from Rust.

### Default Copying Behavior

When you assign a struct to a new variable or pass it to a function, it is copied:

```kestrel
struct Point {
    var x: Int
    var y: Int
}

func example() {
    let p1 = Point(x: 1, y: 2);
    let p2 = p1;  // p1 is copied to p2
    // Both p1 and p2 are valid and independent
}
```

### Copy in Function Calls

```kestrel
func processPoint(p: Point) {
    // p is a copy of the original
}

func example() {
    let point = Point(x: 10, y: 20);
    processPoint(point);  // point is copied
    // point is still valid here
}
```

### Mutating via `mutating` Parameters

To modify a struct in a function, use `mutating`:

```kestrel
func offset(mutating p: Point, by: Int) {
    p.x = p.x + by;
    p.y = p.y + by;
}

func example() {
    var point = Point(x: 10, y: 20);
    offset(point, by: 5);  // point is modified in place
    // point is now (15, 25)
}
```

## Non-Copyable Structs

Structs can opt out of copy semantics by conforming to `not Copyable`. These types use move semantics instead.

### Declaring Non-Copyable Structs

```kestrel
struct File: not Copyable {
    var handle: Int

    deinit {
        // Close file handle
        close(self.handle);
    }
}
```

### Move Semantics

Non-copyable types are moved, not copied:

```kestrel
func example() {
    let file1 = File.open("data.txt");
    let file2 = file1;  // file1 is moved to file2
    // file1 is no longer valid
    // file2 now owns the file handle
}
```

### Implicit Non-Copyable

A struct is implicitly non-copyable if it contains a non-copyable field:

```kestrel
struct Container {
    var file: File  // File is not Copyable
    var data: Array[Byte]
}
// Container is implicitly not Copyable
```

### Non-Copyable and Consuming

Use `consuming` parameters to take ownership:

```kestrel
func closeFile(consuming f: File) {
    // f is destroyed at the end of this function
}

func example() {
    let file = File.open("data.txt");
    closeFile(file);
    // file is no longer valid
}
```

## Examples

### Simple Data Structure

```kestrel
struct Person {
    var name: String
    var age: Int

    func greet() -> String {
        "Hello, I'm " + self.name
    }
}

func example() {
    let person = Person(name: "Alice", age: 30);
    let greeting = person.greet();
}
```

### Resource Management with RAII

```kestrel
struct Database: not Copyable {
    var connection: Int

    static func connect(url: String) -> Database {
        let conn = openConnection(url);
        Database(connection: conn)
    }

    func query(sql: String) -> Result[Data, Error] {
        // Execute query
    }

    deinit {
        closeConnection(self.connection);
    }
}

func example() {
    let db = Database.connect("localhost:5432");
    let result = db.query("SELECT * FROM users");
    // db.deinit is called automatically when db goes out of scope
}
```

### Nested Structs

```kestrel
struct Game {
    var score: Int

    struct Player {
        var name: String
        var lives: Int
    }

    struct Level {
        var number: Int
        var difficulty: String
    }
}

func example() {
    let player = Game.Player(name: "Bob", lives: 3);
    let level = Game.Level(number: 1, difficulty: "easy");
}
```

### Generic Container

```kestrel
struct Stack[T] {
    var items: Array[T]

    init() {
        self.items = Array[T]();
    }

    mutating func push(item: T) {
        self.items.append(item);
    }

    mutating func pop() -> Optional[T] {
        if self.items.isEmpty() {
            .None
        } else {
            .Some(self.items.removeLast())
        }
    }

    func isEmpty() -> Bool {
        self.items.isEmpty()
    }
}

func example() {
    var stack = Stack[Int]();
    stack.push(1);
    stack.push(2);
    stack.push(3);

    match stack.pop() {
        .Some(value) => print(value),  // 3
        .None => print("empty")
    }
}
```

### Protocol Conformance with Generics

```kestrel
protocol Container {
    type Item
    var count: Int { get }
    func isEmpty() -> Bool
}

struct Wrapper[T]: Container {
    type Item = T
    var items: Array[T]

    var count: Int {
        self.items.count()
    }

    func isEmpty() -> Bool {
        self.items.isEmpty()
    }
}
```

## Grammar

```
StructDeclaration → Attributes? Visibility? STRUCT Identifier TypeParameters? ProtocolConformance? StructBody

TypeParameters → LBRACKET TypeParameter (COMMA TypeParameter)* RBRACKET

TypeParameter → Identifier (EQUALS Type)?

ProtocolConformance → COLON ProtocolList

ProtocolList → TypePath (COMMA TypePath)*

StructBody → LBRACE StructMember* RBRACE

StructMember → FieldDeclaration
             | InitDeclaration
             | DeinitDeclaration
             | MethodDeclaration
             | StructDeclaration
             | TypeAliasDeclaration

FieldDeclaration → Attributes? Visibility? STATIC? (VAR | LET) Identifier COLON Type (EQUALS Expression)?
                 | Attributes? Visibility? STATIC? VAR Identifier COLON Type ComputedBody

ComputedBody → LBRACE Expression RBRACE
             | LBRACE GetterClause SetterClause? RBRACE

GetterClause → GET CodeBlock

SetterClause → SET CodeBlock

InitDeclaration → Attributes? Visibility? INIT ParameterList CodeBlock

DeinitDeclaration → DEINIT CodeBlock

MethodDeclaration → Attributes? Visibility? STATIC? MethodModifier? FUNC Identifier TypeParameters? ParameterList ReturnType? WhereClause? CodeBlock

MethodModifier → MUTATING | CONSUMING
```

## Implementation Notes

### Parser

- Structs are declaration items
- `struct` keyword required
- Type parameters are optional `[T, U]`
- Protocol conformance follows name: `struct Foo: Protocol`
- Body contains fields, methods, initializers, and nested declarations

### Semantic Analysis

- `StructSymbol` with nested symbols for fields, methods, and nested types
- Implicit memberwise initializer generated if no explicit `init`
- Fields have `TypedBehavior` for their declared type
- Methods have implicit `self` parameter (not declared in signature)
- `mutating` methods require mutable receiver
- `consuming` methods take ownership of `self`

### Type Checking

- Structs are nominal types (identified by name, not structure)
- Copy-by-default unless `not Copyable`
- Generic structs are monomorphized at compile time
- Protocol conformance checked via duck typing on methods

### Codegen

- Struct layout determined by field order
- Methods are static functions with implicit `self` parameter
- Memberwise init generates constructor code
- `deinit` inserted at scope exit points
- Copy operations generated for copyable structs
- Move operations for non-copyable structs
