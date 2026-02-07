# Default Function Parameters Design

## Overview

Default function parameters allow functions, initializers, and subscripts to specify default values for parameters. When a caller omits an argument, the default expression is used instead.

**Motivation**: Reduces boilerplate by eliminating the need for multiple overloads that differ only in optional arguments. Common in Swift, TypeScript, Python, Kotlin, and other modern languages.

## Syntax

```kestrel
// Basic default parameter
func greet(name: String = "World") {
    print("Hello, {name}!")
}

// Multiple defaults
func createPoint(x: Int = 0, y: Int = 0) -> Point {
    return Point { x, y }
}

// With labeled parameters
func createUser(with name: String = "Anonymous", age: Int = 18) -> User {
    return User { name, age }
}

// Mixed required and default (required must come first)
func divide(numerator: Int, denominator: Int = 1) -> Int {
    return numerator / denominator
}

// With access modes
func modify(mutating value: Int = 0) { }

// In initializers
struct Rectangle {
    var width: Int
    var height: Int

    init(width: Int = 100, height: Int = 100) {
        self.width = width
        self.height = height
    }
}

// In subscripts
subscript(index: Int = 0) -> Element {
    return elements[index]
}

// Any expression allowed (evaluated at call site)
func log(timestamp: Date = Date.now(), message: String) {
    print("[{timestamp}] {message}")
}
```

## Semantic Behavior

### Evaluation Semantics
- Default expressions are evaluated **at the call site**, not at function definition time
- Each call with an omitted argument evaluates the default expression fresh
- This allows expressions like `Date.now()` or mutable collections to work correctly

### Type Checking
- The default expression must be assignable to the parameter type
- Type inference works normally within the default expression
- Generic type parameters are in scope for default expressions

### Parameter Ordering
- **Required parameters must come before default parameters**
- `func foo(a: Int = 0, b: Int)` is a compile error
- This simplifies call-site resolution and matches most language conventions

### No Parameter References
- Default expressions **cannot reference other parameters**
- `func foo(a: Int, b: Int = a)` is a compile error
- Default expressions are evaluated in the enclosing scope, not in a parameter scope

### Scope of Default Expressions
- Default expressions have access to:
  - Type parameters of the function
  - Symbols visible at the function definition site
  - Static members of types in scope
- Default expressions do NOT have access to:
  - Other parameters of the same function
  - Instance members (for static functions)

## Interaction with Other Features

### Overloading
- Function signatures for overload resolution include parameter labels but **exclude default values**
- Two functions with identical signatures except for default values are **duplicate errors**:
  ```kestrel
  func foo(x: Int) { }        // Signature: foo(x:)
  func foo(x: Int = 0) { }    // Signature: foo(x:) - DUPLICATE ERROR
  ```
- Call resolution considers defaults: `foo()` matches `func foo(x: Int = 0)`

### Call Resolution
- Arguments can be omitted for parameters with defaults
- Omitted arguments must be at the end (no "skipping" middle parameters without labels)
- With labeled parameters, any defaulted parameter can be omitted:
  ```kestrel
  func make(width: Int = 10, height: Int = 20) { }

  make()                    // OK: width=10, height=20
  make(width: 5)            // OK: width=5, height=20
  make(height: 15)          // OK: width=10, height=15
  make(width: 5, height: 15) // OK: width=5, height=15
  ```

### Initializers
- Initializers support default parameters identically to functions
- Explicit initializer with defaults does NOT suppress implicit memberwise init
- Memberwise init generated only if no explicit init exists

### Subscripts
- Subscripts support default parameters identically to functions
- Common use case: `collection[index: Int = 0]`

### Closures
- Closure types do NOT support default parameters
- `|x: Int = 0| x + 1` is invalid syntax
- Closures are called by type signature, not by name

### Generic Type Parameters
- Generic type parameters are in scope for default expressions:
  ```kestrel
  func make[T: Default](value: T = T.default()) -> T {
      return value
  }
  ```
- The default expression must be valid for all valid type arguments

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Required param after default | "required parameter cannot follow a parameter with a default value" |
| Default references other param | "default value cannot reference other parameters" |
| Default type mismatch | "cannot convert value of type 'X' to parameter type 'Y'" |
| Duplicate signature with defaults | "duplicate function 'name'" |
| Invalid expression in default | (standard expression errors apply) |

## Edge Cases

### Empty function calls
```kestrel
func allDefaults(x: Int = 0, y: Int = 0) { }
allDefaults()  // Valid: x=0, y=0
```

### Defaults with complex expressions
```kestrel
func process(data: Array[Int] = Array[Int].new()) { }
// Each call gets a fresh empty array
```

### Defaults in protocol requirements
```kestrel
protocol Greeter {
    func greet(name: String = "World")  // Default in protocol
}

struct MyGreeter: Greeter {
    func greet(name: String = "World") { }  // Must match default? TBD
}
```
**Decision**: Protocol default values are optional syntax sugar. Conforming implementations may have different defaults or no defaults.

### Visibility of default expressions
- Default expressions are part of the function's public interface
- If a function is public, its default expressions must only reference public symbols

## Open Questions (Resolved)

| Question | Resolution |
|----------|------------|
| Call-site vs definition-site evaluation? | Call-site - each call evaluates fresh |
| Literals only or any expression? | Any expression allowed |
| Required after default allowed? | No - compile error |
| Can defaults reference other params? | No - compile error |
| How do defaults affect overloading? | Signature ignores defaults; duplicates are errors |
| Closures support defaults? | No |
| Initializers/subscripts support defaults? | Yes, identically to functions |
