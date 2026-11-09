# Computed Properties

Computed properties provide a way to define properties whose values are calculated rather than stored. They use getter and setter blocks to compute values on access and optionally respond to value changes.

## Syntax

### Getter-Only (Shorthand)

The most common form - a single expression that computes the value:

```kestrel
var isEmpty: Bool { self.count == 0 }
```

This is shorthand for the explicit getter form:

```kestrel
var isEmpty: Bool { get { self.count == 0 } }
```

### Getter and Setter

For read-write computed properties:

```kestrel
var celsius: Float64 {
    get { (self.fahrenheit - 32) * 5 / 9 }
    set { self.fahrenheit = newValue * 9 / 5 + 32 }
}
```

The setter receives an implicit `newValue` parameter of the property's type. It is
`consuming`: the setter owns the new value and can move it into storage (so
a `not Copyable` property is settable). The caller copies a value it still
needs; a temporary moves in for free. Subscript setters work the same way.

### Static Computed Properties

Type-level computed properties:

```kestrel
struct Int64 {
    public static var zero: Int64 { Int64(value: 0) }
    public static var maxValue: Int64 { Int64(value: 9223372036854775807) }
}
```

### Protocol Property Requirements

Protocols declare property requirements using `{ get }` or `{ get set }`:

```kestrel
protocol Numeric {
    static var zero: Self { get }
    static var one: Self { get }
}

protocol Container {
    var count: Int { get }
    var isEmpty: Bool { get }
}

protocol MutableContainer: Container {
    var count: Int { get set }
}
```

### `modifying get` and `modifying ref`

A plain `get` borrows `self`. Spell `modifying get` (or `modifying ref`) when
reading the property has to change `self`, such as a memoizing cache or a
read counter:

```kestrel
struct Primes {
    var limit: Int64
    var cache: Optional[Array[Int64]]

    var all: Array[Int64] {
        modifying get {
            if let .Some(found) = self.cache { return found.clone(); }
            let found = sieve(self.limit);
            self.cache = .Some(found.clone());
            found
        }
    }
}
```

`modifying` means the accessor changes `self`, so every read needs a mutable
base, just like calling a `mutating` method. Reading through a `let`, an
immutable field or a shared `&T` is E203, E204 or E207. `modifying ref`
lends `&T` with a mutating receiver; `mutating ref` stays the accessor that
lends `&mutating T`. A `modifying` read provider can be paired with `set` or
`mutating ref` for writes, and subscripts accept the same forms.

Rules:

- One read provider per member: `get`, `modifying get`, `ref` or
  `modifying ref` (E619).
- `modifying` accessors need `self`, so static and global members can't use
  them (E636), and protocols can't declare them (E621). A protocol can
  require a plain `ref` / `mutating ref` place, though (see
  [references](references.md)).
- A `modifying` read can't witness a protocol's `{ get }` requirement, since
  the requirement promises reads don't mutate (E477).

### Failable Properties

A property whose place may not exist is declared failable with `?` after the
name, matching `init?`:

```kestrel
struct Stack[T] {
    var items: Array[T]

    var top?: T {
        get {
            if self.items.isEmpty { return null; }
            .Some(self.items(at: self.items.count - 1))
        }
    }
}

if let .Some(t) = stack.top { ... }
```

Reads produce `Optional[T]`: a `get` body returns `T?`, and a `ref` body
lends `Optional[&T]`. Subscripts use the same marker after their parameter
list: `subscript(checked index: Int64)? -> T`. Assigning to a failable
place is E214, since it may not exist, unless the member declares `set`
(below). A failable member needs a read accessor (E637).

To write through a failable place, declare a `mutating ref` that lends
`Optional[&mutating T]`, and unwrap it with a `&mutating` payload binder:

```kestrel
var first?: T {
    ref { ... }                    // lends Optional[&T] for reads
    mutating ref {                 // lends Optional[&mutating T]
        if self.items.isEmpty { return null; }
        let r = &mutating self.items(at: 0);
        .Some(r)
    }
}

if let .Some(&mutating v) = stack.first { v += 1; }
```

A failable member may also declare `set`. Its `newValue` is `Optional[T]`,
so the setter decides what assigning `null` means, and plain `=` calls it
even when a `mutating ref` is declared too. `Dictionary` uses this:
`dict(key) = v` inserts or replaces, and `dict(key) = null` removes.
Compound assignment (`dict(key) += 1`) is still E214, since it needs the
current value, which may be missing:

```kestrel
subscript(key: K)? -> V {
    ref { ... }
    mutating ref { ... }      // for `if let .Some(&mutating v) = dict(k)`
    set {                     // for `dict(k) = …`; newValue: Optional[V]
        match newValue {
            .Some(v) => { let _ = self.insert(key, v); },
            .None => { let _ = self.remove(key); },
        }
    }
}
```

A match whose arms have a `&mutating` binder reads the failable member
through its `mutating ref`, so the receiver must be mutable; every other
read goes through `get` or `ref`. Without a `mutating ref`, a `&mutating`
binder is E210.

### Throwing Properties

A computed property or subscript declared `T throws E` is a throwing place,
the `Result` twin of a failable one. `get` returns `Result[T, E]`, `ref`
lends `Result[&T, E]`, and `mutating ref` lends `Result[&mutating T, E]`:

```kestrel
var first: T throws BadIndex {
    ref {
        if self.items.isEmpty { throw BadIndex.Empty; }
        let r = &self.items(at: 0);
        .Ok(r)
    }
}

let x = try stack.first;                          // in a throwing function
if let .Ok(&mutating v) = stack.first { v += 1; } // with a `mutating ref`
```

Like a failable place it can't declare `set` (E637) or be assigned (E214).

### Property Observers

`willSet` and `didSet` run code around every write to a stored property:

```kestrel
struct Thermostat {
    var log: Array[Int64]

    var target: Int64 = 20 {
        willSet { self.log.append(newValue); }
        didSet {
            if self.target > 30 { self.target = 30; }
            let _ = oldValue;
        }
    }
}
```

- The property stays stored. Its memberwise-init parameter, layout,
  initializers and `@derive`d conformances are unchanged.
- `willSet` sees the incoming `newValue`; `didSet` sees `oldValue`, which is
  copied out only when the `didSet` body names it (so a `not Copyable`
  property can be observed as long as `didSet` doesn't ask for it).
- Every write from outside runs the observers once: `t.target = 5`,
  `t.target += 1`, mutating method calls on the property, writes to its
  fields (`t.pos.x = 1`) and subscript writes (`t.items(at: 0) = 1`), all
  through the get → modify → set writeback.
- An initializer writes its own `self`'s storage directly, and an
  observer's own writes to its property don't re-run the observers. Writes
  to any other instance, even inside an initializer, run them. A closure
  that writes the property runs them too.
- An observed property can't be lent as `&mutating` (E210), because its
  observers could not run.
- `return` is not allowed in `willSet` / `didSet` (E638): both run inside
  one synthesized setter, so it would skip the store.
- Observers belong on an instance stored `var` of a struct or enum. They are
  rejected on `let`, static and global properties, subscripts, failable
  members, protocol members, and when mixed with `get`/`set`/`ref` (E638).

### Lazy Properties

`lazy var` defers a property's initializer to its first read:

```kestrel
struct Document {
    var text: String

    lazy var wordCount: Int64 = self.countWords()
}
```

- The initializer runs once, on the first read, and may use `self`.
- Reads go through a synthesized `modifying get`, so they need a mutable
  base (a `var`, not a `let`).
- Assignment replaces the cached value without running the initializer.
- The property is not a memberwise-init parameter, and every initializer
  starts it uncomputed. Its cache is hidden state: `@derive`d conformances
  and struct patterns ignore it.
- `lazy` needs an instance `var` of a struct or enum, with a type
  annotation and an `= expr` initializer, and no accessors (E639).
- Each read returns a copy of the cached value, so the type must be
  Copyable (E639).

## Rules

### Only `var`

Computed properties must use `var`, not `let`:

```kestrel
// Valid
var isEmpty: Bool { self.count == 0 }

// Invalid - computed properties cannot use let
let isEmpty: Bool { self.count == 0 }  // ERROR
```

> **Note:** The current parser implementation does not enforce this constraint. Using `let` with a computed property body is currently accepted syntactically, though this may change in future versions to align with the language specification.

### Same Visibility for Getter and Setter

Unlike Swift, Kestrel does not support different visibility levels for getters and setters:

```kestrel
// Valid
public var value: Int {
    get { self._value }
    set { self._value = newValue }
}

// NOT supported - no split visibility
public private(set) var value: Int { ... }  // NOT VALID
```

### Implicit Return

Single-expression getter bodies return implicitly, consistent with function behavior:

```kestrel
// Implicit return
var doubled: Int { self.value * 2 }

// Equivalent explicit return
var doubled: Int { get { return self.value * 2 } }
```

### Setter Parameter

The setter receives an implicit `newValue` parameter. Explicit parameter naming is not supported:

```kestrel
// Valid - uses implicit newValue
set { self._value = newValue }

// NOT supported - explicit parameter name
set(value) { self._value = value }  // NOT VALID
```

## Where Allowed

Computed properties can appear in:

- **Structs** - instance and static
- **Enums** - instance and static
- **Protocols** - as requirements (`{ get }` or `{ get set }`)
- **Extensions** - adding computed properties to existing types

```kestrel
struct Point {
    var x: Int
    var y: Int

    var magnitude: Float64 { sqrt(self.x * self.x + self.y * self.y) }
}

enum Optional[T] {
    case Some(T)
    case None

    var isSome: Bool {
        match self {
            .Some(_) => true,
            .None => false
        }
    }
}

extend String {
    var isEmpty: Bool { self.count == 0 }
}
```

## Not Supported

The following features are intentionally not included:

- **Split visibility** (`public private(set)`) - getter and setter share visibility
- **Explicit setter parameter names** (`set(value)`) - always uses `newValue`
- **`let` computed properties** - use `var` with getter-only

## Examples

### Derived Values

```kestrel
struct Rectangle {
    var width: Float64
    var height: Float64

    var area: Float64 { self.width * self.height }
    var perimeter: Float64 { 2 * (self.width + self.height) }
}
```

### Type Constants

```kestrel
struct UInt8 {
    public static var zero: UInt8 { UInt8(value: 0) }
    public static var min: UInt8 { UInt8(value: 0) }
    public static var max: UInt8 { UInt8(value: 255) }
}
```

### Wrapper Properties

```kestrel
struct Temperature {
    private var kelvin: Float64

    var celsius: Float64 {
        get { self.kelvin - 273.15 }
        set { self.kelvin = newValue + 273.15 }
    }

    var fahrenheit: Float64 {
        get { self.celsius * 9 / 5 + 32 }
        set { self.celsius = (newValue - 32) * 5 / 9 }
    }
}
```

### Protocol Conformance

```kestrel
protocol Named {
    var name: String { get }
}

protocol MutableNamed: Named {
    var name: String { get set }
}

struct Person: MutableNamed {
    private var _name: String

    var name: String {
        get { self._name }
        set { self._name = newValue }
    }
}
```

### Enum Computed Properties

```kestrel
enum Result[T, E] {
    case Ok(T)
    case Err(E)

    var isOk: Bool {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }

    var isErr: Bool { not self.isOk }
}
```

## Grammar

```
FieldDeclaration → Attributes? Visibility? STATIC? (VAR | LET) Identifier COLON Type ComputedBody?

ComputedBody → LBRACE Expression RBRACE
             | LBRACE GetterClause SetterClause? RBRACE

GetterClause → GET CodeBlock

SetterClause → SET CodeBlock

ProtocolPropertyRequirement → Attributes? Visibility? STATIC? (VAR | LET) Identifier COLON Type PropertyRequirementBody

PropertyRequirementBody → LBRACE GET RBRACE
                        | LBRACE GET SET RBRACE
```

## Implementation Notes

### Parser

The parser must distinguish between:
1. Stored property: `var x: Int` (no body, ends with semicolon or newline)
2. Computed property (shorthand): `var x: Int { expr }`
3. Computed property (explicit): `var x: Int { get { } set { } }`
4. Protocol requirement: `var x: Int { get }` or `var x: Int { get set }`

### Semantic Analysis

- Computed properties do not allocate storage
- Getter must return a value of the declared type
- Setter receives `newValue` of the declared type
- `self` is available in both getter and setter
- Setter implies the enclosing method must be `mutating` (for structs)

### Type Checking

- Getter body type must match property type
- `newValue` in setter has the property's type
- Protocol conformance: `{ get set }` requirement needs both getter and setter in implementation
