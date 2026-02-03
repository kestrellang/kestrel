# Functions

Functions are fundamental building blocks in Kestrel. They encapsulate reusable logic and can be standalone (module-level) or associated with types (methods).

## Declaration Syntax

### Basic Functions

```kestrel
func greet() {
    // Function body
}

func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_add(a, b)
}
```

Functions use the `func` keyword, followed by a name, parameter list in parentheses, optional return type after `->`, and a body in braces.

### Return Types

If a function has no return type, it implicitly returns `()` (unit/void).

```kestrel
// These are equivalent
func doSomething() { }
func doSomething() -> () { }
```

Return statements are optional for the last expression in a function body:

```kestrel
func multiply(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_mul(a, b)  // Implicit return
}

func divide(a: lang.i64, b: lang.i64) -> lang.i64 {
    return lang.i64_div(a, b);  // Explicit return
}
```

### Expression-Bodied Functions

For functions whose body is a single expression, you can use the shorthand `= expression` syntax instead of a block:

```kestrel
// Expression body (shorthand)
func add(a: lang.i64, b: lang.i64) -> lang.i64 = lang.i64_add(a, b)

// Equivalent block body
func add(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_add(a, b)
}
```

Expression bodies are semantically equivalent to a block with a single trailing expression. They work with all function features:

```kestrel
// With generics
func identity[T](x: T) -> T = x

// With where clauses
func double[T](x: T) -> T where T: Addable = x.add(x)

// Instance methods
struct Point {
    let x: lang.i64
    let y: lang.i64

    func sum() -> lang.i64 = lang.i64_add(self.x, self.y)
}

// Static methods
struct Factory {
    static func zero() -> lang.i64 = 0
}

// Multiline expressions
func max(a: lang.i64, b: lang.i64) -> lang.i64 =
    if lang.i64_gt(a, b) { a }
    else { b }
```

**Rules:**
- Expression bodies require an explicit return type (no type inference)
- Cannot be used with extern functions (which cannot have any body)
- The expression's type must match the declared return type

## Parameters

### Basic Parameters

Parameters are declared with a name and type:

```kestrel
func process(value: lang.i64, count: lang.i64) -> lang.i64 {
    // Use value and count
    lang.i64_mul(value, count)
}
```

### Labeled Parameters (External Names)

Parameters can have external labels used at the call site, separate from the internal name used in the function body:

```kestrel
func send(to recipient: lang.str) {
    // 'recipient' is the internal name
}

func send(from sender: lang.str) {
    // 'sender' is the internal name
}

// Called as:
send(to: "alice@example.com")
send(from: "bob@example.com")
```

Syntax: `externalLabel internalName: Type`

External labels enable:
- More readable call sites
- Function overloading (same name, different labels)
- Self-documenting APIs

### Parameter Access Modes

Kestrel has three parameter access modes that control how arguments are passed and what operations are allowed:

#### 1. Borrowing (Default)

The default mode. The function receives read-only access to the argument.

```kestrel
func readPoint(p: Point) -> lang.i64 {
    p.x  // Can read
}

// Caller retains ownership
let point = Point(x: 1, y: 2);
let x = readPoint(point);
// Can still use point here
```

- Parameter is immutable inside the function
- Caller can still use the value after the call
- Works with `let` or `var` bindings
- Accepts temporaries

#### 2. Mutating

Enables the function to modify the argument. The caller must pass a mutable variable.

```kestrel
func reset(mutating p: Point) {
    p.x = 0;
    p.y = 0;
}

// Must pass a var binding
var point = Point(x: 1, y: 2);
reset(point);  // point is now (0, 0)
```

Rules:
- Keyword `mutating` before parameter name
- Caller must pass a `var` binding, mutable field, or mutable subscript
- Cannot pass `let` bindings or temporaries
- Changes are visible to the caller

```kestrel
// ERROR: Cannot pass let binding to mutating parameter
let immutable = Point(x: 1, y: 2);
reset(immutable);  // Compile error

// ERROR: Cannot pass temporary to mutating parameter
reset(Point(x: 1, y: 2));  // Compile error
```

#### 3. Consuming

The function takes ownership of the argument.

```kestrel
func consume(consuming p: Point) -> lang.i64 {
    p.x  // Can read and modify
}

let point = Point(x: 1, y: 2);
consume(point);
// For Copyable types, point is still usable (a copy was passed)
// For non-Copyable types, point is moved and no longer valid
```

Rules:
- Keyword `consuming` before parameter name
- Parameter is mutable inside the function body
- For Copyable types: a copy is made automatically
- For non-Copyable types: value is moved, caller cannot use it afterward

### Combining Labels and Access Modes

Access mode keywords come before the label (if present):

```kestrel
func offset(mutating point p: Point, by delta: lang.i64) {
    p.x = lang.i64_add(p.x, delta);
    p.y = lang.i64_add(p.y, delta);
}

// Called as:
var pt = Point(x: 1, y: 2);
offset(point: pt, by: 5);
```

## Generic Functions

Functions can be parameterized over types using type parameters:

```kestrel
func identity[T](value: T) -> T {
    value
}

// Explicit type arguments
let x: lang.i64 = identity[lang.i64](42);

// Type inference (when possible)
let y = identity(42);  // T inferred as lang.i64
```

### Multiple Type Parameters

```kestrel
func pair[A, B](a: A, b: B) -> (A, B) {
    (a, b)
}

let p = pair[lang.i64, lang.str](42, "hello");
```

### Generic Constraints

Use `where` clauses to constrain type parameters to protocols:

```kestrel
protocol Equatable {
    func equals(other: Self) -> lang.i1
}

func areEqual[T](a: T, b: T) -> lang.i1 where T: Equatable {
    a.equals(b)
}
```

Multiple constraints:

```kestrel
protocol Display { }
protocol Debug { }

func show[T](value: T) where T: Display and Debug {
    // Can call methods from both protocols
}

// Or separate clauses
func process[A, B](a: A, b: B) where A: Display, B: Debug {
    // ...
}
```

## Function Overloading

Functions can be overloaded by:

1. **Parameter count**: Different number of parameters

```kestrel
func process() { }
func process(x: lang.i64) { }
func process(x: lang.i64, y: lang.i64) { }
```

2. **Parameter labels**: Different external parameter names

```kestrel
func send(to recipient: lang.str) { }
func send(from sender: lang.str) { }
```

**Note**: Kestrel does **not** support overloading by parameter type alone (currently produces an error):

```kestrel
// ERROR: duplicate function signature
func convert(x: lang.i64) -> lang.str { "lang.i64" }
func convert(x: lang.f64) -> lang.str { "float" }
```

## Methods

Methods are functions declared inside `struct`, `enum`, or `extension` blocks. Unlike standalone functions, methods have implicit access to `self`.

### Important: `self` is Implicit

**Unlike Rust**, methods in Kestrel do **not** declare `self` as an explicit parameter.

```kestrel
struct Counter {
    var count: lang.i64

    // CORRECT - self is implicit
    func getValue() -> lang.i64 {
        self.count
    }

    // WRONG - Do NOT write this:
    // func getValue(self) -> lang.i64 { ... }
    // func getValue(&self) -> lang.i64 { ... }
}
```

### Method Receivers

Methods can have different access modes for `self`, controlled by modifiers on the method:

#### Borrowing Methods (Default)

No modifier. The method has read-only access to `self`.

```kestrel
struct Point {
    var x: lang.i64
    var y: lang.i64

    func getX() -> lang.i64 {
        self.x  // Read-only access
    }
}
```

#### Mutating Methods

Use the `mutating` keyword to allow modification of `self`.

```kestrel
struct Counter {
    var count: lang.i64

    mutating func increment() {
        self.count = lang.i64_add(self.count, 1);
    }
}

var counter = Counter(count: 0);
counter.increment();  // Requires var binding
```

Rules:
- Can only be called on `var` bindings
- Cannot be called on `let` bindings or temporaries
- Can modify fields of `self`

#### Consuming Methods

Use the `consuming` keyword to take ownership of `self`.

```kestrel
struct Container {
    var value: lang.i64

    consuming func extract() -> lang.i64 {
        self.value
    }
}

let c = Container(value: 42);
let x = c.extract();
// For Copyable types, c is still usable
// For non-Copyable types, c is consumed
```

### Static Methods

Static methods are associated with the type, not an instance. They do not have access to `self`.

```kestrel
struct Counter {
    var count: lang.i64

    static func create() -> Counter {
        Counter(count: 0)
    }
}

let c = Counter.create();
```

Use cases:
- Factory methods
- Utility functions related to the type
- Type-level operations

## Nested Functions

Functions can be declared inside other functions (though this is not commonly used):

```kestrel
func outer() -> lang.i64 {
    func inner() -> lang.i64 {
        42
    }
    inner()
}
```

## Extern Functions (C FFI)

Extern functions enable calling C code from Kestrel using the `@extern` attribute.

```kestrel
struct MyInt: FFISafe { }
struct Ptr: FFISafe { }

@extern(.C)
func malloc(size: MyInt) -> Ptr

@extern(.C, mangleName: "read")
func readSocket(fd: MyInt, buf: Ptr, count: MyInt) -> MyInt

@extern(.C)
func free(ptr: Ptr)  // Void return
```

### Extern Function Rules

1. **Calling Convention**: Must specify `.C` calling convention
2. **FFI Safety**: All parameter and return types must conform to `FFISafe` protocol
3. **No Body**: Extern functions cannot have a body
4. **No Generics**: Extern functions cannot be generic
5. **Parameter Modes**: Parameters are implicitly `consuming` (passed by value)
   - Cannot use `mutating` on extern function parameters
   - Can optionally write `consuming` explicitly
6. **Mangling**: Use `mangleName:` to specify the C symbol name

```kestrel
// ERROR: Cannot have body
@extern(.C)
func bad(x: MyInt) -> MyInt { x }

// ERROR: Cannot be generic
@extern(.C)
func generic[T](x: T) -> T

// ERROR: mutating not allowed
@extern(.C)
func mutate(mutating x: MyInt) -> MyInt
```

## Visibility

Functions can have visibility modifiers:

```kestrel
public func publicFn() { }      // Visible everywhere
private func privateFn() { }    // Visible only in this file/scope
func internalFn() { }           // Internal by default
```

## Grammar

```
Function ::= Visibility? 'func' IDENTIFIER TypeParameterList? '(' ParameterList? ')' ('->' Type)? (WhereClause)? FunctionBody?

ParameterList ::= Parameter (',' Parameter)*

Parameter ::= AccessMode? (IDENTIFIER IDENTIFIER)? IDENTIFIER ':' Type

AccessMode ::= 'mutating' | 'consuming'

TypeParameterList ::= '[' TypeParameter (',' TypeParameter)* ']'

TypeParameter ::= IDENTIFIER ('=' Type)?

WhereClause ::= 'where' Constraint (',' Constraint)*

Constraint ::= IDENTIFIER ':' ProtocolBound ('and' ProtocolBound)*

FunctionBody ::= Block | '=' Expression

Method ::= MethodModifier? 'func' IDENTIFIER TypeParameterList? '(' ParameterList? ')' ('->' Type)? (WhereClause)? (Block | '=' Expression)?

MethodModifier ::= 'mutating' | 'consuming' | 'static'

ExternFunction ::= '@extern' '(' ExternConvention (',' 'mangleName:' STRING)? ')' 'func' IDENTIFIER '(' ParameterList? ')' ('->' Type)?

ExternConvention ::= '.C'
```

## Examples

### Complete Function with All Features

```kestrel
protocol Addable {
    func add(other: Self) -> Self
}

public func combine[T](
    mutating accumulator acc: T,
    with items: [T],
    using mapper: (T) -> T
) -> T where T: Addable {
    for item in items {
        let transformed = mapper(item);
        acc = acc.add(transformed);
    }
    acc
}
```

### Method Example with All Receiver Types

```kestrel
struct Document {
    var content: lang.str
    var modified: lang.i1

    // Borrowing method - read-only
    func getContent() -> lang.str {
        self.content
    }

    // Mutating method - can modify
    mutating func updateContent(to new: lang.str) {
        self.content = new;
        self.modified = true;
    }

    // Consuming method - takes ownership
    consuming func finalize() -> lang.str {
        self.content
    }

    // Static method - no self
    static func create() -> Document {
        Document(content: "", modified: false)
    }
}
```

### FFI Example

```kestrel
import Prelude

struct CInt: FFISafe { }
struct CString: FFISafe { }

@extern(.C)
func strlen(s: CString) -> CInt

@extern(.C)
func strcmp(s1: CString, s2: CString) -> CInt

@extern(.C, mangleName: "printf")
func printFormatted(format: CString) -> CInt

func useFFI() {
    // Call C functions from Kestrel
    let len = strlen(myCString);
}
```

## Common Patterns

### Builder Pattern with Mutating Methods

```kestrel
struct Builder {
    var value: lang.i64

    mutating func setValue(to v: lang.i64) {
        self.value = v;
    }

    mutating func multiply(by factor: lang.i64) {
        self.value = lang.i64_mul(self.value, factor);
    }

    consuming func build() -> lang.i64 {
        self.value
    }
}

var builder = Builder(value: 1);
builder.setValue(to: 10);
builder.multiply(by: 2);
let result = builder.build();  // 20
```

### Generic Factory Methods

```kestrel
struct Box[T] {
    var value: T

    static func wrap(v: T) -> Box[T] {
        Box(value: v)
    }
}

let intBox = Box.wrap(42);
let strBox = Box.wrap("hello");
```

## See Also

- [Semantics Guide](semantics.md) - Details on parameter modes and memory model
- [Generics](generics.md) - Type parameters and constraints
- [Pattern Matching](pattern-matching.md) - Control flow in function bodies
