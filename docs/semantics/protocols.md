# Protocols

Protocols define interfaces that types can conform to. They declare method signatures without implementations.

## Syntax

```
ProtocolDeclaration → Visibility? PROTOCOL Identifier LBRACE ProtocolMember* RBRACE

ProtocolMember → FunctionDeclaration    // Must NOT have body
```

### Tokens
- `PROTOCOL` - The `protocol` keyword
- `LBRACE` / `RBRACE` - Curly braces `{` `}`
- `Visibility` - Optional visibility modifier

## Examples

### Empty Protocol

```kestrel
protocol Marker { }
```

### Protocol with Methods

```kestrel
protocol Drawable {
    func draw()
    func erase()
}
```

### Protocol with Return Types

```kestrel
protocol DataSource {
    func count() -> Int
    func itemAt(index: Int) -> String
    func isEmpty() -> Bool
}
```

### Protocol with Labeled Parameters

```kestrel
protocol Repository {
    func save(item: Data)
    func load(with id: Int) -> Data
    func delete(at index: Int)
    func find(matching predicate: String) -> Data
}
```

### Protocol with Static Methods

```kestrel
protocol Factory {
    static func create() -> Self
    static func createWith(config: Config) -> Self
}
```

### Protocol with Visibility

```kestrel
public protocol PublicAPI {
    func fetch() -> Data
    func update(data: Data)
}

internal protocol InternalContract {
    func process()
}
```

## Semantic Rules

### Rule 1: Protocol Methods Cannot Have Bodies

Methods in protocols are declarations only—no implementation.

```
ERROR: ProtocolMethodPass error
WHEN: A method in a protocol has a body
WHY: Protocols define interfaces, not implementations
```

**Example (invalid):**
```kestrel
protocol Runnable {
    func run() { }    // ERROR: protocol method 'run' cannot have a body
}
```

**Example (valid):**
```kestrel
protocol Runnable {
    func run()        // OK: no body
}
```

### Rule 2: No Duplicate Method Signatures

Within a protocol, method signatures must be unique.

```
ERROR: Duplicate signature error
WHEN: Two methods have identical signatures
WHY: Would create ambiguity in conformance
```

**Example (invalid):**
```kestrel
protocol Bad {
    func process(x: Int)
    func process(x: Int)    // ERROR: duplicate signature
}
```

**Example (valid - different signatures):**
```kestrel
protocol Good {
    func process(x: Int)
    func process(x: String)    // OK: different type
    func process(with x: Int)  // OK: different label
}
```

### Rule 3: Static Methods Allowed

Protocols can declare static methods.

```kestrel
protocol Buildable {
    static func build() -> Self
    static func buildWith(options: Options) -> Self
}
```

### Rule 4: No Fields in Protocols

Protocols can only contain method declarations—not fields.

```
ERROR: Parse error or semantic error
WHEN: Field declaration inside protocol
WHY: Protocols define behavior, not storage
```

**Example (invalid):**
```kestrel
protocol HasValue {
    let value: Int    // ERROR: protocols cannot have fields
}
```

### Rule 5: No Nested Types in Protocols

Protocols cannot contain nested type declarations.

```
ERROR: Parse error or semantic error
WHEN: Class, struct, or protocol nested in protocol
WHY: Protocols define method contracts only
```

**Example (invalid):**
```kestrel
protocol Container {
    struct Item { }    // ERROR: cannot nest types in protocol
}
```

## Protocol as a Type

Protocols create nominal types:

```kestrel
protocol Printable { }

func print(item: Printable) { }    // Printable as parameter type
func getPrintable() -> Printable { }  // Printable as return type
```

The protocol type is created during the build phase and attached via `TypedBehavior`.

## Protocol Conformance

**Note:** Protocol conformance (implementing a protocol in a struct) is not yet implemented. This section describes the intended design.

### Declaring Conformance

```kestrel
// Future syntax
struct Point: Drawable {
    let x: Int
    let y: Int

    func draw() { }    // Required by Drawable
    func erase() { }   // Required by Drawable
}
```

### Conformance Requirements

A type conforms to a protocol if it provides implementations for all required methods with matching signatures.

## Protocol Scope

Protocols create a scope containing only method declarations:

```
Protocol scope
└── Method declarations (no bodies)
```

### Visibility Scope

For private methods, the protocol is the visibility scope:

```kestrel
public protocol API {
    func publicMethod()
    // private func internalMethod()  // Not typically useful
}
```

## Protocol vs Class/Struct

| Aspect | Protocol | Class/Struct |
|--------|----------|--------------|
| Methods | Signatures only | Full implementations |
| Fields | Not allowed | Allowed |
| Nested types | Not allowed | Allowed |
| Can be instantiated | No | Yes |
| Purpose | Define interface | Provide implementation |

## Formal Semantics

### Protocol Declaration

For `protocol P { methods... }`:

```
Preconditions:
    - All methods must NOT have bodies
    - No duplicate method signatures
    - No field declarations
    - No nested type declarations

Effect:
    - Creates ProtocolSymbol with name P
    - Creates type Ty::Protocol(Arc<ProtocolSymbol>)
    - Adds TypedBehavior with protocol type
    - Creates scope for P
    - Processes all method declarations in P's scope
```

### Type Creation

```rust
let protocol_symbol = Arc::new(ProtocolSymbol::new(name, visibility));
let protocol_type = Ty::protocol(protocol_symbol.clone(), span);
protocol_symbol.add_behavior(TypedBehavior::new(protocol_type, span));
```

### Conformance (Future)

```
conforms(Type, Protocol) iff:
    ∀ method M in Protocol:
        ∃ implementation I in Type:
            signature(I) = signature(M)
```

## Symbol Structure

```rust
ProtocolSymbol {
    name: String,
    visibility_behavior: VisibilityBehavior,
    typed_behavior: TypedBehavior,  // Contains Ty::Protocol(self)
    children: Vec<Symbol>,          // Method declarations only
}
```

## Common Patterns

### Capability Protocol

```kestrel
protocol Identifiable {
    func id() -> String
}

protocol Hashable {
    func hash() -> Int
}

protocol Equatable {
    func equals(other: Self) -> Bool
}
```

### Data Access Protocol

```kestrel
protocol Repository {
    func findById(id: Int) -> Entity
    func findAll() -> (Entity)
    func save(entity: Entity)
    func delete(entity: Entity)
}
```

### Lifecycle Protocol

```kestrel
protocol Lifecycle {
    func initialize()
    func start()
    func stop()
    func destroy()
}
```

### Factory Protocol

```kestrel
protocol Factory {
    static func create() -> Self
    static func createWith(config: Config) -> Self
}
```

### Observer Protocol

```kestrel
protocol Observer {
    func onEvent(event: Event)
    func onError(error: Error)
    func onComplete()
}
```

## Source Location

- **Build/lowering:** `lib/kestrel-semantic-tree-builder/src/builders/protocol.rs`
- **Bind:** `lib/kestrel-semantic-tree-binder/src/binders/protocol.rs`
- **Symbol:** `lib/kestrel-semantic-tree/src/symbol/protocol.rs`
- **Validate (protocol method rules):** `lib/kestrel-semantic-analyzers/src/analyzers/protocol_method/mod.rs`
- **Validate (conformances):** `lib/kestrel-semantic-analyzers/src/analyzers/conformance/mod.rs`
