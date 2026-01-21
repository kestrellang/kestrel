# Closures

Closures are anonymous functions that can capture values from their surrounding scope. They provide a concise way to define inline behavior and are first-class values that can be passed as arguments, returned from functions, and stored in variables or data structures.

## Basic Syntax

### No Parameters

The simplest closure has no parameters and no `in` keyword:

```kestrel
let f: () -> lang.i64 = { 42 }
f()  // Returns 42
```

With explicit empty parameters and `in` keyword:

```kestrel
let f: () -> lang.i64 = { () in 42 }
```

### Single Parameter

With explicit type annotation:

```kestrel
let double: (lang.i64) -> lang.i64 = { (x: lang.i64) in x * 2 }
```

With inferred type (requires context):

```kestrel
let double: (lang.i64) -> lang.i64 = { (x) in x * 2 }
```

### Multiple Parameters

With explicit types:

```kestrel
let add: (lang.i64, lang.i64) -> lang.i64 = { (x: lang.i64, y: lang.i64) in x + y }
```

With inferred types:

```kestrel
let add: (lang.i64, lang.i64) -> lang.i64 = { (x, y) in x + y }
```

Mixed typed and untyped parameters:

```kestrel
let f: (lang.i64, lang.str) -> lang.i64 = { (x: lang.i64, y) in x }
```

## Implicit `it` Parameter

When a closure has exactly one parameter and the expected type is known, you can use the implicit `it` parameter instead of declaring explicit parameters:

```kestrel
func apply(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
    f(x)
}

let result = apply({ it * 2 }, 21)  // Returns 42
```

### Rules for `it`

- `it` is only available when the expected function type has exactly 1 parameter
- Using `it` when arity is 0 or 2+ is an error
- Explicit parameters shadow `it` - you cannot use both
- `it` in nested closures refers to the innermost closure's parameter

```kestrel
// ERROR: it used but arity is 0
let f: () -> lang.i64 = { it }

// ERROR: it used but arity is 2
let g: (lang.i64, lang.i64) -> lang.i64 = { it }

// ERROR: it not available with explicit params
let h: (lang.i64) -> lang.i64 = { (x) in it }

// OK: nested it shadows outer
func apply(f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(10)
}

let f: (lang.i64) -> lang.i64 = {
    let outer = it;
    apply({ it + outer })  // inner `it` is different
}
```

## Multi-Statement Closures

Closures can contain multiple statements. The last expression is the return value:

```kestrel
let compute: (lang.i64, lang.i64) -> lang.i64 = { (x, y) in
    let sum = x + y;
    let doubled = sum * 2;
    let result = doubled + 1;
    result
}
```

Closures support all statement types:

```kestrel
// With mutable variables
let process: (lang.i64) -> lang.i64 = { (x) in
    var acc = 0;
    acc = acc + x;
    acc = acc + x;
    acc
}

// With if expressions
let absolute: (lang.i64) -> lang.i64 = { (x) in
    if x > 0 {
        x
    } else {
        -x
    }
}

// With while loops
let sumTo: (lang.i64) -> lang.i64 = { (n) in
    var i = 0;
    var sum = 0;
    while i < n {
        sum = sum + i;
        i = i + 1;
    }
    sum
}
```

## Capture Semantics

Closures **capture by value** - variables from the enclosing scope are copied into the closure when it's created.

### Basic Captures

```kestrel
func makeAdder(n: lang.i64) -> (lang.i64) -> lang.i64 {
    { (x) in x + n }  // n is captured by value
}

let add10 = makeAdder(10);
add10(5)  // Returns 15
```

### Capture Rules

1. **Immutable captures**: Captured variables are read-only inside the closure
2. **Capture by value**: The value is copied at closure creation time
3. **Multiple captures**: Closures can capture multiple variables
4. **No mutation**: You cannot assign to captured variables

```kestrel
// Capture multiple variables
func makeComplex() -> () -> lang.i64 {
    let a = 1;
    let b = 2;
    let c = 3;
    { a + b + c }
}

// ERROR: cannot mutate captured variable
func test() -> () -> lang.i64 {
    var x = 10;
    {
        x = 20;  // ERROR: cannot assign to captured variable
        x
    }
}

// Capture by value semantics
func test() -> () -> lang.i64 {
    var x = 10;
    let f = { x };  // x=10 is captured
    x = 20;         // mutation doesn't affect closure
    f               // Returns 10, not 20
}
```

### Parameter Shadowing

Closure parameters shadow captured variables with the same name:

```kestrel
func test() {
    let x = 100;
    let f: (lang.i64) -> lang.i64 = { (x) in x + 20 };
    f(22)  // Returns 42, uses parameter x (22), not captured x (100)
}
```

## Trailing Closure Syntax

When a closure is the last argument to a function, it can be written outside the parentheses:

### Only Argument

```kestrel
func apply(f: () -> lang.i64) -> lang.i64 {
    f()
}

// Instead of: apply({ 42 })
apply { 42 }
```

### Last of Multiple Arguments

```kestrel
func fold(initial: lang.i64, f: (lang.i64, lang.i64) -> lang.i64) -> lang.i64 {
    f(initial, 10)
}

// Instead of: fold(0, { (acc, n) in acc + n })
fold(0) { (acc, n) in acc + n }
```

### With Implicit `it`

```kestrel
func transform(x: lang.i64, f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(x)
}

transform(5) { it * 2 }  // Returns 10
```

## Type Inference

Kestrel infers closure types based on context. Type information can flow from:

1. **Expected type** (function parameter, variable annotation, return type)
2. **Closure body** (return type inferred from body expression)

```kestrel
// Parameter types inferred from expected type
let f: (lang.i64) -> lang.i64 = { (x) in x + 1 }

// Return type inferred from body
let g: (lang.i64) -> lang.i64 = { (x: lang.i64) in x * 2 }

// Both inferred from context
func transform(x: lang.i64, f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(x)
}
transform(5, { (x) in x * 2 })  // All types inferred

// ERROR: cannot infer without context
let h = { (x) in x }  // No type annotation or context
```

### Type Inference with `it`

The `it` parameter's type is inferred from the expected function type:

```kestrel
func apply(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
    f(x)
}

// Type of `it` inferred as lang.i64 from parameter type
apply({ it * 2 }, 21)
```

## Closures as Values

Closures are first-class values that can be stored, passed, and returned.

### Stored in Variables

```kestrel
let f: (lang.i64) -> lang.i64 = { it * 2 };
let result = f(21)  // Returns 42
```

### Passed as Arguments

```kestrel
func apply(x: lang.i64, f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(x)
}

apply(10, { it + 1 })  // Returns 11
```

### Returned from Functions

```kestrel
func makeMultiplier(n: lang.i64) -> (lang.i64) -> lang.i64 {
    { (x) in x * n }
}

let times3 = makeMultiplier(3);
times3(14)  // Returns 42
```

### Stored in Structs

```kestrel
struct Handler {
    let action: (lang.i64) -> lang.i64
}

let h = Handler(action: { it * 2 });
(h.action)(21)  // Returns 42
```

Note: Parentheses around field access are required when calling: `(h.action)(arg)`.

### Stored in Enums

```kestrel
enum Action {
    case Transform(f: (lang.i64) -> lang.i64)
    case NoOp
}

let action = Action.Transform(f: { it * 2 });

match action {
    .Transform(f: f) => f(21),
    .NoOp => 0
}
```

### Generic Containers

```kestrel
struct Provider[T] {
    let provide: () -> T
}

let p = Provider[lang.i64](provide: { 42 });
(p.provide)()  // Returns 42

struct Transform[T, U] {
    let transform: (T) -> U
}

let t = Transform[lang.i64, lang.i64](transform: { it * 2 });
(t.transform)(21)  // Returns 42
```

## Nested Closures

Closures can contain other closures, enabling currying and higher-order patterns:

```kestrel
// Closure returning a closure
func makeAdder() -> (lang.i64) -> (lang.i64) -> lang.i64 {
    { (x) in { (y) in x + y } }
}

let add = makeAdder();
let add10 = add(10);
add10(5)  // Returns 15
```

### Nested Captures

Inner closures can capture from outer closures:

```kestrel
let f: (lang.i64) -> (lang.i64) -> lang.i64 = {
    (x) in {
        (y) in x + y  // inner closure captures outer's x parameter
    }
};
```

### Nested `it` Shadowing

Each closure level has its own `it`:

```kestrel
func apply(f: (lang.i64) -> lang.i64) -> lang.i64 {
    f(5)
}

let f: (lang.i64) -> lang.i64 = {
    let outer = it;           // outer closure's it
    apply({ it + outer })     // inner closure's it is different
}
```

## Immediate Invocation

Closures can be invoked immediately where they're defined:

```kestrel
// No parameters
let x = { 42 }()  // Returns 42

// With parameters
let sum = { (x: lang.i64, y: lang.i64) in x + y }(10, 20)  // Returns 30

// For scoping
let result = {
    let a = 10;
    let b = 20;
    a + b
}()  // Returns 30, a and b not visible outside
```

## Type Checking

The compiler validates closure types against expected types:

```kestrel
// ERROR: arity mismatch - too few parameters
let f: (lang.i64, lang.i64) -> lang.i64 = { (x) in x }

// ERROR: arity mismatch - too many parameters
let g: (lang.i64) -> lang.i64 = { (x, y) in x + y }

// ERROR: return type mismatch
let h: (lang.i64) -> lang.str = { (x) in x * 2 }

// ERROR: parameter type mismatch
let i: (lang.i64) -> lang.i64 = { (x: lang.str) in 42 }

// ERROR: closure assigned to non-function type
let j: lang.i64 = { 42 }
```

## Parameter Mutability

Closure parameters are immutable by default:

```kestrel
// ERROR: cannot assign to closure parameter
let f: (lang.i64) -> lang.i64 = { (x) in
    x = 10;  // ERROR
    x
}
```

To modify values, use local mutable variables:

```kestrel
let f: (lang.i64) -> lang.i64 = { (x) in
    var temp = x;
    temp = temp * 2;
    temp
}
```

## Grammar

```
closure ::= '{' closure_params? body '}'

closure_params ::= '(' param_list ')' 'in'
                 | '(' ')' 'in'

param_list ::= param (',' param)*

param ::= identifier (':' type)?

body ::= statement* expression?
       | expression

// Note: When no closure_params are provided and the body uses `it`,
// the implicit single-parameter form is used
```

### Syntax Notes

- No `in` keyword when there are no explicit parameters
- Empty `()` requires `in` keyword
- Parameters can mix typed and untyped forms
- The `in` keyword separates parameters from body
- The body is a block that can contain statements and a trailing expression

## Higher-Order Functions

Closures enable functional programming patterns:

### Composition

```kestrel
func compose(
    f: (lang.i64) -> lang.i64,
    g: (lang.i64) -> lang.i64
) -> (lang.i64) -> lang.i64 {
    { (x) in g(f(x)) }
}

let add10 = { (x: lang.i64) in x + 10 };
let double = { (x: lang.i64) in x * 2 };
let composed = compose(add10, double);
composed(11)  // (11 + 10) * 2 = 42
```

### Apply Twice

```kestrel
func applyTwice(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
    f(f(x))
}

applyTwice({ (x) in x + 10 }, 22)  // (22 + 10) + 10 = 42
```

## Common Patterns

### Factory Functions

```kestrel
func makeCounter(start: lang.i64) -> () -> lang.i64 {
    var count = start;
    { () in
        let current = count;
        count = count + 1;
        current
    }
}
```

Note: This pattern is conceptual. Actual implementation depends on Kestrel's capture semantics allowing mutable captures, which is currently not supported.

### Callbacks

```kestrel
struct Button {
    let onClick: () -> ()
}

let button = Button(onClick: {
    print("Button clicked!")
})
```

### Configuration

```kestrel
func configure(builder: Builder, with: (Builder) -> ()) -> Builder {
    with(builder);
    builder
}

configure(myBuilder) { (b) in
    b.setWidth(100);
    b.setHeight(200);
}
```

## Implementation Notes

### Current Status

- Closures are implemented and fully functional
- Capture by value is the only capture mode
- No explicit return type annotation syntax (return type is inferred)
- The `it` parameter is available for single-parameter closures

### Future Enhancements

- Explicit return type syntax: `{ (x) -> ReturnType in body }`
- Capture lists for controlling what gets captured
- Mutable captures or capture by reference
