# Functions

Functions are callable units of code that can accept parameters and return values. Kestrel supports function overloading with Swift-style labeled parameters.

## Syntax

```
FunctionDeclaration → Visibility? STATIC? FUNC Identifier ParameterList ReturnType? FunctionBody?

ParameterList → LPAREN (Parameter (COMMA Parameter)* COMMA?)? RPAREN

Parameter → Label? BindName COLON Type

Label → Identifier

BindName → Identifier

ReturnType → ARROW Type

FunctionBody → LBRACE RBRACE
```

### Tokens
- `FUNC` - The `func` keyword
- `STATIC` - The `static` keyword
- `ARROW` - The `->` symbol
- `COLON` - The `:` character
- `LPAREN` / `RPAREN` - Parentheses
- `LBRACE` / `RBRACE` - Braces

## Parameter Forms

### Simple Parameter

```kestrel
func process(x: Int) { }
//           ^ bind name only
```

- No external label
- Called as: `process(42)`

### Labeled Parameter

```kestrel
func send(to recipient: String) { }
//        ^^ ^^^^^^^^^
//        |  bind name (internal)
//        label (external)
```

- `to` is the label (used by callers)
- `recipient` is the bind name (used inside function)
- Called as: `send(to: "Alice")`

### Underscore Label (Suppress Label)

```kestrel
func add(_ x: Int, _ y: Int) { }
```

- `_` as label means "no label required"
- Called as: `add(1, 2)`

## Examples

### Basic Functions

```kestrel
// No parameters, no return
func doSomething() { }

// With return type
func getValue() -> Int { }

// With parameters
func add(a: Int, b: Int) -> Int { }

// Multiple parameters
func format(name: String, age: Int, active: Bool) -> String { }
```

### Labeled Parameters

```kestrel
// Swift-style labels
func move(from source: Point, to dest: Point) { }
// Called as: move(from: start, to: end)

func insert(element: Int, at index: Int) { }
// Called as: insert(element: 5, at: 0)

// Same bind name, different label
func copy(from source: String, to target: String) { }
// Called as: copy(from: a, to: b)
```

### Static Functions

```kestrel
struct Math {
    static func square(x: Int) -> Int { }
    static func max(a: Int, b: Int) -> Int { }
}
```

### Function Overloading

```kestrel
// Overload by parameter count
func print() { }
func print(message: String) { }
func print(message: String, level: Int) { }

// Overload by parameter types
func process(x: Int) { }
func process(x: String) { }
func process(x: Bool) { }

// Overload by labels
func connect(to server: String) { }
func connect(using config: Config) { }
func connect(with options: Options) { }
```

## Semantic Rules

### Rule 1: Functions Outside Protocols Must Have Bodies

Functions declared at module level, in structs, or in structs must include a body.

```
ERROR: FunctionBodyPass error
WHEN: Function outside protocol has no body
WHY: Implementation is required for non-protocol functions
```

**Example (invalid):**
```kestrel
func compute() -> Int    // ERROR: function 'compute' requires a body

struct Calculator {
    func add(a: Int, b: Int) -> Int    // ERROR: requires a body
}
```

**Example (valid):**
```kestrel
func compute() -> Int { }    // OK: has body

struct Calculator {
    func add(a: Int, b: Int) -> Int { }    // OK: has body
}
```

### Rule 2: Protocol Methods Must Not Have Bodies

Functions inside protocols are method signatures only.

```
ERROR: ProtocolMethodPass error
WHEN: Protocol method has a body
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
    func stop()       // OK: no body
}
```

### Rule 3: No Duplicate Signatures

Within the same scope, two functions cannot have identical signatures.

```
ERROR: Duplicate signature error
WHEN: Two functions have the same name AND same parameter types
WHY: Would create ambiguity during overload resolution
```

**Signature includes:**
- Function name
- Parameter labels (if any)
- Parameter types (in order)

**Example (invalid):**
```kestrel
func add(x: Int, y: Int) { }
func add(x: Int, y: Int) { }    // ERROR: duplicate function signature

func process(a: String) { }
func process(b: String) { }     // ERROR: same signature (labels don't differ)
```

**Example (valid - different signatures):**
```kestrel
func add(x: Int, y: Int) { }
func add(x: Int, y: Int, z: Int) { }    // OK: different arity

func process(x: Int) { }
func process(x: String) { }              // OK: different types

func send(to x: String) { }
func send(using x: String) { }           // OK: different labels
```

### Rule 4: Static Modifier Only in Types

The `static` keyword can only be used inside struct or protocol declarations.

```
ERROR: StaticContextPass error
WHEN: static function at module level
WHY: static only makes sense relative to an enclosing type
```

**Example (invalid):**
```kestrel
module MyApp

static func utility() { }    // ERROR: static modifier only allowed inside struct or protocol
```

**Example (valid):**
```kestrel
struct Helper {
    static func utility() { }    // OK: inside struct
}

struct Math {
    static func abs(x: Int) -> Int { }    // OK: inside struct
}
```

### Rule 5: Visibility Consistency

Public functions cannot expose less-visible types in parameters or return type.

```
ERROR: VisibilityConsistencyPass error
WHEN: Public function uses private/internal/fileprivate types
WHY: External code couldn't use the function due to inaccessible types
```

**Example (invalid):**
```kestrel
private struct Secret { }

public func getSecret() -> Secret { }              // ERROR: exposes private type
public func processSecret(s: Secret) { }           // ERROR: exposes private type in parameter
```

See [Visibility](visibility.md) for complete rules.

## Function Signatures

A function's signature determines its identity for overloading:

```rust
struct CallableSignature {
    name: String,
    labels: Vec<Option<String>>,    // External labels
    param_types: Vec<SignatureType>,
}
```

### Signature Components

| Component | Affects Signature | Example |
|-----------|-------------------|---------|
| Name | Yes | `add` vs `subtract` |
| Labels | Yes | `send(to:)` vs `send(using:)` |
| Param types | Yes | `(Int)` vs `(String)` |
| Param count | Yes | `()` vs `(Int)` vs `(Int, Int)` |
| Bind names | No | `f(x: Int)` = `f(y: Int)` |
| Return type | No | `f() -> Int` = `f() -> String` |
| Visibility | No | `public f()` = `private f()` |
| Static | No* | *But static functions are in different scope |

### Signature Examples

```kestrel
// Different signatures (valid overloads):
func f()                    // Signature: f()
func f(x: Int)              // Signature: f(_: Int)
func f(x: Int, y: Int)      // Signature: f(_: Int, _: Int)
func f(x: String)           // Signature: f(_: String)
func f(with x: Int)         // Signature: f(with: Int)
func f(using x: Int)        // Signature: f(using: Int)

// Same signature (invalid duplicates):
func f(a: Int)              // Signature: f(_: Int)
func f(b: Int)              // Signature: f(_: Int) - DUPLICATE!
```

## Callable Behavior

Functions have a `CallableBehavior` that stores:

```rust
struct CallableBehavior {
    parameters: Vec<CallableParameter>,
    return_type: Ty,
}

struct CallableParameter {
    label: Option<Name>,     // External label
    bind_name: Name,         // Internal name
    ty: Ty,                  // Parameter type
}
```

## Function Data Behavior

Additional function metadata in `FunctionDataBehavior`:

```rust
struct FunctionDataBehavior {
    has_body: bool,      // Whether function has implementation
    is_static: bool,     // Whether function is static
}
```

## Default Return Type

Functions without an explicit return type default to `()` (Unit):

```kestrel
func doWork() { }           // Returns ()
func doWork() -> () { }     // Equivalent
```

## Formal Semantics

### Function Declaration

For function `func f(p₁: T₁, ..., pₙ: Tₙ) -> R`:

```
Preconditions:
    - Each Tᵢ must be a valid, resolvable type
    - R must be a valid, resolvable type (or () if omitted)
    - No duplicate signature in same scope
    - Has body unless in protocol
    - No body if in protocol
    - static only if inside struct/protocol

Effect:
    - Creates FunctionSymbol with name f
    - Adds CallableBehavior with parameters and return type
    - Adds FunctionDataBehavior with has_body, is_static
    - Registers signature for duplicate detection
```

### Signature Equality

```
signature(f) = signature(g) iff:
    f.name = g.name AND
    length(f.params) = length(g.params) AND
    ∀i: f.params[i].label = g.params[i].label AND
    ∀i: f.params[i].type = g.params[i].type
```

### Overload Resolution

When multiple functions match a name, select by:
1. Match parameter count
2. Match parameter labels
3. Match parameter types

If exactly one function matches, select it. If zero or multiple match, error.

## Source Location

- **Build/lowering:** `lib/kestrel-semantic-tree-builder/src/builders/function.rs`
- **Bind:** `lib/kestrel-semantic-tree-binder/src/binders/function.rs`
- **Body resolution:** `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs`
- **Symbol:** `lib/kestrel-semantic-tree/src/symbol/function.rs`
- **Callable behavior:** `lib/kestrel-semantic-tree/src/behavior/callable.rs`
- **Function data:** `lib/kestrel-semantic-tree/src/behavior/function_data.rs`
- **Signature:** `lib/kestrel-semantic-tree/src/behavior/callable.rs` (`CallableSignature`)
- **Duplicate signature diagnostic:** `lib/kestrel-semantic-tree-binder/src/diagnostics/declaration.rs`
- **Validate:** `lib/kestrel-semantic-analyzers/src/analyzers/function_body/mod.rs`
- **Protocol validation:** `lib/kestrel-semantic-analyzers/src/analyzers/protocol_method/mod.rs`
- **Static validation:** `lib/kestrel-semantic-analyzers/src/analyzers/static_context/mod.rs`
