# Closures

Closures are anonymous functions that can capture values from their surrounding scope. They provide a concise way to define inline behavior and are first-class values that can be passed as arguments, returned from functions, and stored in variables or data structures.

## Basic Syntax

### No Parameters

The simplest closure has no parameters and no `in` keyword:

```kestrel
let f: () -> Int64 = { 42 }
f()  // Returns 42
```

With explicit empty parameters and `in` keyword:

```kestrel
let f: () -> Int64 = { () in 42 }
```

### Single Parameter

With explicit type annotation:

```kestrel
let double: (Int64) -> Int64 = { (x: Int64) in x * 2 }
```

With inferred type (requires context):

```kestrel
let double: (Int64) -> Int64 = { (x) in x * 2 }
```

### Multiple Parameters

With explicit types:

```kestrel
let add: (Int64, Int64) -> Int64 = { (x: Int64, y: Int64) in x + y }
```

With inferred types:

```kestrel
let add: (Int64, Int64) -> Int64 = { (x, y) in x + y }
```

Mixed typed and untyped parameters:

```kestrel
let f: (Int64, String) -> Int64 = { (x: Int64, y) in x }
```

## Implicit `it` Parameter

When a closure has exactly one parameter and the expected type is known, you can use the implicit `it` parameter instead of declaring explicit parameters:

```kestrel
func apply(f: (Int64) -> Int64, x: Int64) -> Int64 {
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
let f: () -> Int64 = { it }

// ERROR: it used but arity is 2
let g: (Int64, Int64) -> Int64 = { it }

// ERROR: it not available with explicit params
let h: (Int64) -> Int64 = { (x) in it }

// OK: nested it shadows outer
func apply(f: (Int64) -> Int64) -> Int64 {
    f(10)
}

let f: (Int64) -> Int64 = {
    let outer = it;
    apply({ it + outer })  // inner `it` is different
}
```

## Multi-Statement Closures

Closures can contain multiple statements. The last expression is the return value:

```kestrel
let compute: (Int64, Int64) -> Int64 = { (x, y) in
    let sum = x + y;
    let doubled = sum * 2;
    let result = doubled + 1;
    result
}
```

Closures support all statement types:

```kestrel
// With mutable variables
let process: (Int64) -> Int64 = { (x) in
    var acc = 0;
    acc = acc + x;
    acc = acc + x;
    acc
}

// With if expressions
let absolute: (Int64) -> Int64 = { (x) in
    if x > 0 {
        x
    } else {
        -x
    }
}

// With while loops
let sumTo: (Int64) -> Int64 = { (n) in
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

Closures **capture by value** - for `Copyable` types, variables from the enclosing scope are copied into the closure when it's created. Non-`Copyable` values are *moved* into the closure instead (see below).

### Basic Captures

```kestrel
func sumWithBase(base: Int64) -> Int64 {
    let addBase = { (x: Int64) in x + base };  // base captured by value
    addBase(5) + addBase(10)
}
```

### Capture Rules

1. **Immutable captures**: Captured variables are read-only inside the closure
2. **Capture by value**: The value is copied (or moved, if non-`Copyable`) at closure creation time
3. **Multiple captures**: Closures can capture multiple variables
4. **No mutation**: You cannot assign to captured variables
5. **No escape**: A capturing closure cannot outlive the function that created it (see below)

```kestrel
// Capture multiple variables
func complexSum() -> Int64 {
    let a = 1;
    let b = 2;
    let c = 3;
    let f = { a + b + c };
    f()
}

// ERROR: cannot mutate captured variable
func mutateCapture() {
    var x = 10;
    let f = {
        x = 20;  // ERROR[E603]: cannot assign to captured variable
        x
    };
    let _ = f;
}

// Capture by value semantics
func snapshot() -> Int64 {
    var x = 10;
    let f = { x };  // x=10 is captured
    x = 20;         // mutation doesn't affect closure
    f()             // Returns 10, not 20
}
```

### Capturing Non-Copyable Values

Capturing a non-`Copyable` value can't copy it — it **moves** the value into the closure environment. The original binding is invalid afterwards; using it is a use-after-move error (E500):

```kestrel
struct Res: not Copyable {
    var id: Int64;
    func peek() -> Int64 { self.id }
    deinit { }
}

func useAfterCapture() {
    let r = Res(id: 7);
    let f = { () in r.peek() };  // r moved into f's environment
    let x = r.peek();            // ERROR[E500]: use of moved value 'r'
    let _ = f; let _ = x;
}
```

A closure may be called more than once, but it owns exactly one copy of each captured value — so moving a captured non-`Copyable` value *out* of the closure body (returning it, or passing it to a `consuming` parameter) is rejected (E506):

```kestrel
func moveOut() {
    let r = Res(id: 1);
    let f = { () in r };  // ERROR[E506]: cannot move captured value 'r' out of a closure
    let _ = f;
}
```

### Capturing Closures Cannot Escape

A capturing closure's environment lives in the stack frame where the closure was created, so the closure is **escape-checked** (via the same provenance analysis as references): it cannot be returned from the enclosing function, even laundered through a `let` binding or a struct field (E494). Non-capturing closures are unaffected.

```kestrel
func makeAdder(n: Int64) -> (Int64) -> Int64 {
    let f = { (x: Int64) in x + n };
    f  // ERROR[E494]: cannot return this closure: it captures local `n`,
       //              which does not outlive the call
}
```

Capturing closures can still be called locally and passed *down* as arguments — they just can't flow *up* out of their defining frame.

### Parameter Shadowing

Closure parameters shadow captured variables with the same name:

```kestrel
func test() {
    let x = 100;
    let f: (Int64) -> Int64 = { (x) in x + 20 };
    f(22)  // Returns 42, uses parameter x (22), not captured x (100)
}
```

## Trailing Closure Syntax

When a closure is the last argument to a function, it can be written outside the parentheses:

### Only Argument

```kestrel
func apply(f: () -> Int64) -> Int64 {
    f()
}

// Instead of: apply({ 42 })
apply { 42 }
```

### Last of Multiple Arguments

```kestrel
func fold(initial: Int64, f: (Int64, Int64) -> Int64) -> Int64 {
    f(initial, 10)
}

// Instead of: fold(0, { (acc, n) in acc + n })
fold(0) { (acc, n) in acc + n }
```

### With Implicit `it`

```kestrel
func transform(x: Int64, f: (Int64) -> Int64) -> Int64 {
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
let f: (Int64) -> Int64 = { (x) in x + 1 }

// Return type inferred from body
let g: (Int64) -> Int64 = { (x: Int64) in x * 2 }

// Both inferred from context
func transform(x: Int64, f: (Int64) -> Int64) -> Int64 {
    f(x)
}
transform(5, { (x) in x * 2 })  // All types inferred

// ERROR: cannot infer without context
let h = { (x) in x }  // No type annotation or context
```

### Type Inference with `it`

The `it` parameter's type is inferred from the expected function type:

```kestrel
func apply(f: (Int64) -> Int64, x: Int64) -> Int64 {
    f(x)
}

// Type of `it` inferred as Int64 from parameter type
apply({ it * 2 }, 21)
```

## Closures as Values

Closures are first-class values that can be stored, passed, and returned.

### Stored in Variables

```kestrel
let f: (Int64) -> Int64 = { it * 2 };
let result = f(21)  // Returns 42
```

### Passed as Arguments

```kestrel
func apply(x: Int64, f: (Int64) -> Int64) -> Int64 {
    f(x)
}

apply(10, { it + 1 })  // Returns 11
```

### Returned from Functions

Only **non-capturing** closures can be returned. A closure that captures locals or parameters cannot escape its defining function (E494 — see [Capturing Closures Cannot Escape](#capturing-closures-cannot-escape)):

```kestrel
func makeTripler() -> (Int64) -> Int64 {
    { (x) in x * 3 }   // OK: captures nothing
}

func makeMultiplier(n: Int64) -> (Int64) -> Int64 {
    { (x) in x * n }   // ERROR[E494]: captures `n`, cannot be returned
}
```

### Stored in Structs

```kestrel
struct Handler {
    let action: (Int64) -> Int64
}

let h = Handler(action: { it * 2 });
(h.action)(21)  // Returns 42
```

Note: Parentheses around field access are required when calling: `(h.action)(arg)`.

### Stored in Enums

```kestrel
enum Action {
    case Transform(f: (Int64) -> Int64)
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

let p = Provider[Int64](provide: { 42 });
(p.provide)()  // Returns 42

struct Transform[T, U] {
    let transform: (T) -> U
}

let t = Transform[Int64, Int64](transform: { it * 2 });
(t.transform)(21)  // Returns 42
```

## Nested Closures

Closures can contain other closures.

### Nested Captures

Inner closures can capture from outer closures, as long as the inner closure doesn't escape the outer one. Note that currying (`{ (x) in { (y) in x + y } }`) is rejected: the inner closure captures the outer closure's parameter `x` and would be *returned* from the outer closure's frame, which the escape check forbids (E494).

```kestrel
func apply(f: () -> Int64) -> Int64 {
    f()
}

// OK: the inner closure captures `x` but is only passed down, not returned
let f: (Int64) -> Int64 = { (x) in
    apply({ x + 1 })
};

// ERROR[E494]: inner closure captures `x` and escapes the outer closure
let g: (Int64) -> (Int64) -> Int64 = { (x) in { (y) in x + y } };
```

### Nested `it` Shadowing

Each closure level has its own `it`:

```kestrel
func apply(f: (Int64) -> Int64) -> Int64 {
    f(5)
}

let f: (Int64) -> Int64 = {
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
let sum = { (x: Int64, y: Int64) in x + y }(10, 20)  // Returns 30

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
let f: (Int64, Int64) -> Int64 = { (x) in x }

// ERROR: arity mismatch - too many parameters
let g: (Int64) -> Int64 = { (x, y) in x + y }

// ERROR: return type mismatch
let h: (Int64) -> String = { (x) in x * 2 }

// ERROR: parameter type mismatch
let i: (Int64) -> Int64 = { (x: String) in 42 }

// ERROR: closure assigned to non-function type
let j: Int64 = { 42 }
```

## Parameter Mutability

Closure parameters are immutable by default:

```kestrel
// ERROR: cannot assign to closure parameter
let f: (Int64) -> Int64 = { (x) in
    x = 10;  // ERROR
    x
}
```

To modify values, use local mutable variables:

```kestrel
let f: (Int64) -> Int64 = { (x) in
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
    f: (Int64) -> Int64,
    g: (Int64) -> Int64
) -> (Int64) -> Int64 {
    { (x) in g(f(x)) }
}

let add10 = { (x: Int64) in x + 10 };
let double = { (x: Int64) in x * 2 };
let composed = compose(add10, double);
composed(11)  // (11 + 10) * 2 = 42
```

### Apply Twice

```kestrel
func applyTwice(f: (Int64) -> Int64, x: Int64) -> Int64 {
    f(f(x))
}

applyTwice({ (x) in x + 10 }, 22)  // (22 + 10) + 10 = 42
```

## Common Patterns

### Factory Functions

```kestrel
func makeCounter(start: Int64) -> () -> Int64 {
    var count = start;
    { () in
        let current = count;
        count = count + 1;
        current
    }
}
```

Note: This pattern is conceptual. It requires both mutable captures (not supported — E603) and returning a capturing closure (not supported — E494).

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
- Capture by value is the only capture mode; non-`Copyable` values are moved into the environment (use-after-capture is E500, moving a capture back out is E506)
- Capturing closures are escape-checked via provenance analysis — they cannot leave their defining frame (E494)
- No explicit return type annotation syntax (return type is inferred)
- The `it` parameter is available for single-parameter closures

### Future Enhancements

- Explicit return type syntax: `{ (x) -> ReturnType in body }`
- Capture lists for controlling what gets captured
- Mutable captures or capture by reference
