# Subscripts

Subscripts provide a way to access elements of a collection, list, or sequence using call-like syntax. They are similar to computed properties but accept parameters, enabling indexed or keyed access patterns like `array(0)` or `dictionary("key")`.

## Syntax

### Getter-Only (Shorthand)

The simplest form - a single expression that computes the value:

```kestrel
public subscript(index: Int) -> T {
    self.storage.buffer(unchecked: index)
}
```

This is shorthand for the explicit getter form:

```kestrel
public subscript(index: Int) -> T {
    get { self.storage.buffer(unchecked: index) }
}
```

### Getter and Setter

For read-write subscripts:

```kestrel
public subscript(index: Int) -> T {
    get { self.storage.buffer(unchecked: index) }
    set {
        self.ensureUnique()
        self.storage.buffer(unchecked: index) = newValue
    }
}
```

The setter receives an implicit `newValue` parameter of the subscript's return type.

### Labeled Parameters

Subscripts support argument labels for clarity and overloading:

```kestrel
public subscript(safe index: Int) -> Optional[T] {
    get {
        if index >= 0 and index < self.count {
            .Some(self.storage.buffer(unchecked: index))
        } else {
            .None
        }
    }
    set {
        if index >= 0 and index < self.count {
            if let value = newValue {
                self.ensureUnique()
                self.storage.buffer(unchecked: index) = value
            }
        }
    }
}

public subscript(wrapping index: Int) -> T {
    get {
        let n = self.count
        let wrapped = ((index % n) + n) % n
        self.storage.buffer(unchecked: wrapped)
    }
    set {
        let n = self.count
        let wrapped = ((index % n) + n) % n
        self.ensureUnique()
        self.storage.buffer(unchecked: wrapped) = newValue
    }
}

public subscript(unchecked index: Int) -> T {
    get { self.storage.buffer(unchecked: index) }
    set {
        self.ensureUnique()
        self.storage.buffer(unchecked: index) = newValue
    }
}
```

Usage:
```kestrel
array(0)              // default subscript (if defined)
array(safe: 0)        // labeled subscript
array(wrapping: -1)   // wrapping access
array(unchecked: 0)   // unchecked access
```

### Multiple Parameters

Subscripts can accept multiple parameters:

```kestrel
public subscript(row: Int, column: Int) -> T {
    get { self.storage(row * self.columns + column) }
    set { self.storage(row * self.columns + column) = newValue }
}
```

Usage:
```kestrel
matrix(0, 1) = 42
let value = matrix(row: 0, column: 1)  // with labels
```

### Generic Subscripts

Subscripts can be generic:

```kestrel
public subscript[K](key: K) -> Optional[V] where K: Hashable {
    get { self.lookup(key: key) }
    set { self.insert(key: key, value: newValue) }
}
```

### Static Subscripts

Type-level subscripts for accessing type-associated data:

```kestrel
struct Cache {
    private static var storage: Dictionary[String, Any]

    public static subscript(key: String) -> Optional[Any] {
        get { Self.storage(key) }
        set { Self.storage(key) = newValue }
    }
}
```

Usage:
```kestrel
Cache("user") = currentUser
let cached = Cache("user")
```

### Protocol Subscript Requirements

Protocols declare subscript requirements using `{ get }` or `{ get set }`:

```kestrel
protocol Collection {
    type Element
    type Index

    subscript(index: Index) -> Element { get }
}

protocol MutableCollection: Collection {
    subscript(index: Index) -> Element { get set }
}
```

## Rules

### Subscript Keyword

Subscripts use the `subscript` keyword:

```kestrel
// Valid
public subscript(index: Int) -> T { ... }

// Invalid - subscript is not a method
public func subscript(index: Int) -> T { ... }  // ERROR: subscript is a keyword
```

### Same Visibility for Getter and Setter

Like computed properties, getter and setter share the same visibility:

```kestrel
// Valid
public subscript(index: Int) -> T {
    get { ... }
    set { ... }
}

// NOT supported - no split visibility
public private(set) subscript(index: Int) -> T { ... }  // NOT VALID
```

### Implicit Return

Single-expression getter bodies return implicitly:

```kestrel
// Implicit return
public subscript(index: Int) -> T { self.data(index) }

// Equivalent explicit return
public subscript(index: Int) -> T { get { return self.data(index) } }
```

### Setter Parameter

The setter receives an implicit `newValue` parameter:

```kestrel
// Valid - uses implicit newValue
set { self.data(index) = newValue }

// NOT supported - explicit parameter name
set(value) { self.data(index) = value }  // NOT VALID
```

### Overloading

Subscripts can be overloaded by:
- Different parameter types
- Different parameter labels
- Different number of parameters

```kestrel
struct Array[T] {
    // By index
    public subscript(index: Int) -> T { ... }

    // By range
    public subscript(range: Range[Int]) -> Slice[T] { ... }

    // Safe access with label
    public subscript(safe index: Int) -> Optional[T] { ... }
}
```

### No Default Parameter Values

Subscript parameters cannot have default values:

```kestrel
// NOT supported
public subscript(index: Int = 0) -> T { ... }  // NOT VALID
```

## Where Allowed

Subscripts can appear in:

- **Structs** - instance and static
- **Enums** - instance and static
- **Protocols** - as requirements (`{ get }` or `{ get set }`)
- **Extensions** - adding subscripts to existing types

```kestrel
struct Matrix[T] {
    private var data: Array[T]
    private var rows: Int
    private var columns: Int

    public subscript(row: Int, column: Int) -> T {
        get { self.data(row * self.columns + column) }
        set { self.data(row * self.columns + column) = newValue }
    }
}

enum JSON {
    case Object(Dictionary[String, JSON])
    case Array(Array[JSON])
    case String(String)
    case Number(Float64)
    case Bool(Bool)
    case Null

    public subscript(key: String) -> Optional[JSON] {
        match self {
            .Object(let dict) => dict(key),
            _ => .None
        }
    }

    public subscript(index: Int) -> Optional[JSON] {
        match self {
            .Array(let arr) => arr(safe: index),
            _ => .None
        }
    }
}

extension String {
    public subscript(index: Int) -> Optional[Char] {
        self.char(at: index)
    }

    public subscript(range: Range[Int]) -> Optional[StringSlice] {
        self.slice(range)
    }
}
```

## Not Supported

The following features are intentionally not included:

- **Split visibility** (`public private(set)`) - getter and setter share visibility
- **Explicit setter parameter names** (`set(value)`) - always uses `newValue`
- **Default parameter values** - all parameters must be provided
- **Variadic parameters** - subscripts require fixed parameter counts

## Examples

### Collection Access

```kestrel
struct Array[T] {
    private var storage: Buffer[T]
    private var count: Int

    // Default subscript - panics on out of bounds
    public subscript(index: Int) -> T {
        get {
            if index < 0 or index >= self.count {
                panic("Array index out of bounds")
            }
            self.storage(unchecked: index)
        }
        set {
            if index < 0 or index >= self.count {
                panic("Array index out of bounds")
            }
            self.ensureUnique()
            self.storage(unchecked: index) = newValue
        }
    }

    // Safe subscript - returns Optional
    public subscript(safe index: Int) -> Optional[T] {
        get {
            if index >= 0 and index < self.count {
                .Some(self.storage(unchecked: index))
            } else {
                .None
            }
        }
    }
}
```

### Dictionary Access

```kestrel
struct Dictionary[K, V] where K: Hashable {
    // Key access
    public subscript(key: K) -> Optional[V] {
        get { self.get(key: key) }
        set {
            match newValue {
                .Some(let value) => self.insert(key: key, value: value),
                .None => self.remove(key: key)
            }
        }
    }

    // Default value access
    public subscript(key: K, default defaultValue: V) -> V {
        get {
            match self.get(key: key) {
                .Some(let value) => value,
                .None => defaultValue
            }
        }
        set {
            self.insert(key: key, value: newValue)
        }
    }
}
```

Usage:
```kestrel
var dict = Dictionary[String, Int]()
dict("count") = 42
let value = dict("count")           // Optional[Int]
let withDefault = dict("missing", default: 0)  // Int
```

### Multi-dimensional Access

```kestrel
struct Grid[T] {
    private var data: Array[T]
    public var width: Int
    public var height: Int

    public subscript(x: Int, y: Int) -> T {
        get { self.data(y * self.width + x) }
        set { self.data(y * self.width + x) = newValue }
    }

    public subscript(safe x: Int, y: Int) -> Optional[T] {
        get {
            if x >= 0 and x < self.width and y >= 0 and y < self.height {
                .Some(self.data(y * self.width + x))
            } else {
                .None
            }
        }
    }
}
```

### Protocol Conformance

```kestrel
protocol Indexable {
    type Element
    type Index

    subscript(index: Index) -> Element { get }
}

protocol MutableIndexable: Indexable {
    subscript(index: Index) -> Element { get set }
}

struct Vector[T]: MutableIndexable {
    type Element = T
    type Index = Int

    private var data: Array[T]

    public subscript(index: Int) -> T {
        get { self.data(index) }
        set { self.data(index) = newValue }
    }
}
```

### Slice and Range Access

```kestrel
struct Array[T] {
    // Range subscript
    public subscript(range: Range[Int]) -> ArraySlice[T] {
        ArraySlice(array: self, start: range.start, end: range.end)
    }

    // Safe range subscript
    public subscript(safe range: Range[Int]) -> Optional[ArraySlice[T]] {
        if range.start >= 0 and range.end <= self.count {
            .Some(ArraySlice(array: self, start: range.start, end: range.end))
        } else {
            .None
        }
    }
}
```

Usage:
```kestrel
let array = [1, 2, 3, 4, 5]
let slice = array(1..4)        // [2, 3, 4]
let safe = array(safe: 0..10)  // .None (out of bounds)
```

## Grammar

```
SubscriptDeclaration → Attributes? Visibility? STATIC? SUBSCRIPT GenericParams?
                       LPAREN ParameterList RPAREN ARROW Type WhereClause? SubscriptBody

SubscriptBody → LBRACE Expression RBRACE
              | LBRACE GetterClause SetterClause? RBRACE

GetterClause → GET CodeBlock

SetterClause → SET CodeBlock

ParameterList → Parameter (COMMA Parameter)*

Parameter → Label? Identifier COLON Type

Label → Identifier

ProtocolSubscriptRequirement → Attributes? Visibility? STATIC? SUBSCRIPT GenericParams?
                               LPAREN ParameterList RPAREN ARROW Type WhereClause?
                               SubscriptRequirementBody

SubscriptRequirementBody → LBRACE GET RBRACE
                         | LBRACE GET SET RBRACE
```

## Implementation Notes

### Parser

The parser must:
1. Recognize the `subscript` keyword as a declaration starter
2. Parse generic parameters after `subscript` if present
3. Parse parameter list in parentheses (like function parameters)
4. Parse return type after `->`
5. Parse optional where clause
6. Distinguish between shorthand body `{ expr }` and explicit `{ get { } set { } }`
7. Handle protocol requirements with `{ get }` or `{ get set }`

### Semantic Analysis

- Subscripts do not allocate storage (like computed properties)
- Getter must return a value of the declared return type
- Setter receives `newValue` of the declared return type
- `self` is available in both getter and setter
- Setter implies the enclosing method must be `mutating` (for structs)
- Subscript parameters are immutable within the body

### Type Checking

- Getter body type must match return type
- `newValue` in setter has the return type
- Protocol conformance: `{ get set }` requirement needs both getter and setter
- Overload resolution considers parameter labels and types

### Call Site Transformation

At the call site, subscript access is transformed:
- `array(0)` → getter call with `(index: 0)`
- `array(0) = value` → setter call with `(index: 0)` and `newValue = value`
- `array(safe: 0)` → labeled getter call with `(safe: 0)`
