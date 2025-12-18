# Closures

Closures are anonymous functions that can capture variables from their enclosing scope. They are first-class values with function types.

## Syntax

```
ClosureExpr → LBRACE ClosureBody RBRACE

ClosureBody → ClosureParams IN Statement* Expr?
            | Statement* Expr?

ClosureParams → LPAREN RPAREN
              | LPAREN Param (COMMA Param)* RPAREN

Param → Identifier (COLON Type)?
```

### Tokens
- `LBRACE` / `RBRACE` - Braces `{` `}`
- `LPAREN` / `RPAREN` - Parentheses `(` `)`
- `IN` - The `in` keyword
- `COLON` - Colon `:`
- `COMMA` - Comma `,`

## Forms

### No Parameters

Closures with no parameters omit the parameter list entirely:

```kestrel
{ print("hello") }
{ 42 }
```

### Implicit `it` Parameter

When a closure has no explicit parameters, the implicit `it` identifier is available. It represents the single parameter when the expected type has arity 1:

```kestrel
numbers.map { it * 2 }
strings.filter { it.len() > 0 }
```

### Explicit Parameters

Parameters are declared in parentheses followed by `in`:

```kestrel
{ (x) in x * 2 }
{ (x, y) in x + y }
{ (a, b, c) in a + b + c }
```

### Parameters with Type Annotations

Type annotations are optional. When omitted, types are inferred from context:

```kestrel
// Types inferred from context
{ (x) in x * 2 }

// Explicit types
{ (x: Int) in x * 2 }
{ (x: Int, y: Int) in x + y }

// Mixed (some inferred, some explicit)
{ (x: Int, y) in x + y }
```

### Multi-Statement Closures

Closures can contain multiple statements. The last expression is implicitly returned:

```kestrel
{ (x, y) in
    let sum = x + y
    let doubled = sum * 2
    doubled
}
```

No explicit `return` keyword is needed - the final expression becomes the return value.

## Trailing Closure Syntax

When a closure is the last argument to a function call, it can be written outside the parentheses:

```kestrel
// Standard call syntax
numbers.map({ it * 2 })

// Trailing closure syntax
numbers.map { it * 2 }

// With other arguments
numbers.reduce(0) { (acc, n) in acc + n }

// Closure as only argument - parentheses can be omitted
runLater { print("done") }
```

## Type

Closures have function types. The type is determined by the parameter types and return type:

```kestrel
{ 42 }                        // () -> Int
{ it * 2 }                    // (Int) -> Int (when used with Int context)
{ (x: Int) in x * 2 }         // (Int) -> Int
{ (x: Int, y: Int) in x + y } // (Int, Int) -> Int
{ (s: String) in s.len() }    // (String) -> Int
```

Closures are assignable to variables with compatible function types:

```kestrel
let double: (Int) -> Int = { it * 2 }
let add: (Int, Int) -> Int = { (x, y) in x + y }
```

## Implicit `it` Parameter

The `it` identifier is a special implicit parameter available in closures without explicit parameters.

### Availability Rules

1. `it` is in scope when the closure has no `(params) in` prefix
2. `it` represents the single parameter when expected type has arity 1
3. Using `it` when arity is not 1 produces an error

```kestrel
// Valid uses of `it`
numbers.map { it * 2 }           // Ok: expected (Int) -> T
strings.filter { it.len() > 0 }  // Ok: expected (String) -> Bool

// `it` available but not used - valid for any arity
runLater { print("hi") }         // Ok: expected () -> T, `it` unused
{ 42 }()                         // Ok: `it` unused

// Invalid uses of `it`
runLater { it }                  // Error: `it` used but arity is 0
numbers.reduce(0) { it }         // Error: `it` used but arity is 2
```

### Shadowing in Nested Closures

In nested closures, inner `it` shadows outer `it`:

```kestrel
outer.map {
    let x = it              // outer's element
    inner.map { it * x }    // inner's element (shadows outer)
}
```

### `it` Not Available with Explicit Parameters

When explicit parameters are declared, `it` is not in scope:

```kestrel
numbers.map { (n) in n * 2 }     // Ok: uses explicit param
numbers.map { (n) in it * 2 }    // Error: `it` not in scope
```

## Captures

Closures can reference variables from their enclosing scope. These variables are "captured" by the closure.

### Capture by Value

All captures are by value (copied). The closure receives an immutable copy of the captured variable at the time the closure is created:

```kestrel
var count = 0
let f = { count + 1 }    // captures copy of `count`
count = 10
f()                      // returns 1, not 11
```

### Captured Variables are Immutable

Captured variables cannot be mutated inside the closure:

```kestrel
var x = 0
let f = { x = 1 }        // Error: cannot assign to captured variable `x`
```

### What Gets Captured

A variable is captured if:
1. It is referenced inside the closure body
2. It is defined outside the closure (in an enclosing scope)
3. It is not a parameter of the closure

```kestrel
let multiplier = 2
let numbers = [1, 2, 3]

// `multiplier` is captured, `it` is a parameter
numbers.map { it * multiplier }
```

### Capture Analysis

The compiler performs capture analysis to identify all captured variables:

```kestrel
func makeCounter() -> () -> Int {
    var count = 0
    { 
        count = count + 1  // Error: cannot mutate capture
        count
    }
}
```

## Closure Parameters

### Parameter Mutability

Closure parameters are immutable by default, consistent with function parameters:

```kestrel
{ (x) in
    x = 1    // Error: cannot assign to immutable parameter `x`
    x * 2
}
```

### Parameter Type Inference

When type annotations are omitted, parameter types are inferred from the expected type:

```kestrel
// Expected type: (Int, Int) -> Int
let add: (Int, Int) -> Int = { (x, y) in x + y }
// x and y inferred as Int

// Inferred from function parameter
func apply(f: (String) -> Int) { }
apply { (s) in s.len() }  // s inferred as String
```

## Immediate Invocation

Closures can be immediately invoked, which serves as a scoping mechanism:

```kestrel
let result = {
    let x = computeA()
    let y = computeB()
    x + y
}()
```

This is not special syntax - it's a closure expression followed by a call expression `()`.

## Examples

### Basic Closures

```kestrel
// No parameters
let greet = { print("Hello") }
greet()

// With implicit `it`
let double = { it * 2 }
double(21)  // returns 42

// With explicit parameters
let add = { (x: Int, y: Int) in x + y }
add(1, 2)  // returns 3
```

### Higher-Order Functions

```kestrel
// Map
let doubled = numbers.map { it * 2 }

// Filter
let evens = numbers.filter { it % 2 == 0 }

// Reduce
let sum = numbers.reduce(0) { (acc, n) in acc + n }

// Chained
let result = numbers
    .filter { it > 0 }
    .map { it * 2 }
    .reduce(0) { (acc, n) in acc + n }
```

### Closures as Return Values

```kestrel
func makeMultiplier(factor: Int) -> (Int) -> Int {
    { it * factor }  // captures `factor`
}

let triple = makeMultiplier(factor: 3)
triple(10)  // returns 30
```

### Closures in Data Structures

```kestrel
struct Button {
    var onClick: () -> ()
}

let button = Button(onClick: { print("clicked") })
button.onClick()
```

---

## Semantic Errors

### ItUsedWithWrongArity

```
When:     `it` is referenced but expected closure arity is not 1
Why:      `it` only represents a single parameter
Message:  "`it` can only be used when closure has exactly 1 parameter, but {n} expected"
```

**Example:**
```kestrel
runLater { it }              // Error: 0 parameters expected
numbers.reduce(0) { it }     // Error: 2 parameters expected
```

### ItNotInScope

```
When:     `it` is referenced in a closure with explicit parameters
Why:      Explicit parameters replace the implicit `it`
Message:  "`it` is not in scope; closure has explicit parameters"
```

**Example:**
```kestrel
numbers.map { (n) in it * 2 }  // Error: use `n` instead
```

### CannotAssignToCapturedVariable

```
When:     Assignment to a variable captured from enclosing scope
Why:      Captures are by value and immutable
Message:  "cannot assign to captured variable `{name}`"
```

**Example:**
```kestrel
var x = 0
let f = { x = 1 }  // Error
```

### CannotAssignToClosureParameter

```
When:     Assignment to a closure parameter
Why:      Closure parameters are immutable
Message:  "cannot assign to immutable parameter `{name}`"
```

**Example:**
```kestrel
let f = { (x) in x = 1 }  // Error
```

### CannotInferClosureParameterType

```
When:     Closure parameter type cannot be inferred
Why:      No expected type available and no annotation provided
Message:  "cannot infer type for closure parameter `{name}`; add a type annotation"
```

**Example:**
```kestrel
let f = { (x) in x }  // Error: what is x's type?
```

### ClosureArityMismatch

```
When:     Closure has different arity than expected type
Why:      Parameter count must match function type
Message:  "closure has {actual} parameters but {expected} expected"
```

**Example:**
```kestrel
let f: (Int, Int) -> Int = { (x) in x }  // Error: expected 2 params
```

### ClosureReturnTypeMismatch

```
When:     Closure body type doesn't match expected return type  
Why:      Return type must be compatible with function type
Message:  "closure returns `{actual}` but `{expected}` expected"
```

**Example:**
```kestrel
let f: (Int) -> String = { it * 2 }  // Error: returns Int, not String
```

---

## Semantic Representation

### ClosureExpr

```rust
pub struct ClosureExpr {
    /// Explicit parameters, if any. None means implicit `it` style.
    pub params: Option<Vec<ClosureParam>>,
    
    /// Statements in the closure body
    pub body: Vec<Statement>,
    
    /// Final expression (implicit return value)
    pub tail_expr: Option<Box<Expr>>,
}

pub struct ClosureParam {
    pub name: Identifier,
    pub ty: Option<TypeExpr>,  // None if inferred
}
```

### ClosureBehavior

Attached after semantic analysis:

```rust
pub struct ClosureBehavior {
    /// Variables captured from enclosing scope
    pub captures: Vec<Capture>,
    
    /// Resolved parameters with inferred types
    pub resolved_params: Vec<ResolvedClosureParam>,
    
    /// Resolved return type
    pub return_type: Ty,
    
    /// Whether `it` was referenced in the body
    pub uses_it: bool,
}

pub struct ResolvedClosureParam {
    pub name: Identifier,
    pub ty: Ty,
}
```

### Capture

```rust
pub struct Capture {
    /// The captured variable's symbol
    pub symbol_id: SymbolId,
    
    /// Name of the captured variable
    pub name: Identifier,
    
    /// Type of the captured variable
    pub ty: Ty,
    
    /// How the variable is captured
    pub kind: CaptureKind,
}

pub enum CaptureKind {
    /// Immutable copy (default, only option currently)
    Value,
    
    // Future options:
    // Reference,        // Immutable borrow
    // MutableReference, // Mutable borrow  
    // Move,             // Ownership transfer
}
```

---

## Grammar Summary

```
// Closure expression
closure_expr = "{" closure_body "}"

// Body with optional params
closure_body = closure_params "in" statements tail_expr?
             | statements tail_expr?

// Parameter list
closure_params = "(" ")"
               | "(" param ("," param)* ")"

// Single parameter  
param = IDENT (":" type)?

// Statements and final expression
statements = statement*
tail_expr = expr
```

---

## Source Location

- **Parser:** `lib/kestrel-parser/src/expr/closure.rs` (to be created)
- **Syntax tree:** `lib/kestrel-syntax-tree/src/expr.rs`
- **Semantic tree:** `lib/kestrel-semantic-tree/src/expr/closure.rs` (to be created)
- **Closure behavior:** `lib/kestrel-semantic-tree/src/behavior/closure.rs` (to be created)
- **Capture analysis:** `lib/kestrel-semantic-tree-binder/src/capture.rs` (to be created)
- **Type inference:** `lib/kestrel-semantic-type-inference/src/closure.rs` (to be created)
