# Expression-Bodied Functions Design

## Overview

Expression-bodied functions provide a concise syntax for functions whose body consists of a single expression. Instead of writing a full block body with braces, developers can use `= expression` to define the function body inline.

This feature reduces boilerplate for simple functions and improves code readability for one-liner implementations.

## Syntax

```kestrel
// Basic inline function
func add(a: Int, b: Int) -> Int = lang.i64_add(a, b)

// With generics
func identity[T](x: T) -> T = x

// With where clause
func compare[T](a: T, b: T) -> Bool where T: Equatable = a.equals(b)

// Multi-line expression (expression spans multiple lines)
func max(a: Int, b: Int) -> Int =
    if a > b { a }
    else { b }

// Instance method
struct Point {
    let x: Int
    let y: Int

    func sum() -> Int = lang.i64_add(x, y)
}

// Protocol with default implementation
protocol Describable {
    func description() -> String = "default"
}
```

## Semantic Behavior

### Equivalence
An expression-bodied function is semantically equivalent to a block-bodied function with a single trailing expression:

```kestrel
// These are equivalent:
func add(a: Int, b: Int) -> Int = lang.i64_add(a, b)
func add(a: Int, b: Int) -> Int { lang.i64_add(a, b) }
```

### Return Type
- **Required**: Expression-bodied functions must have an explicit return type annotation
- The expression's type must match the declared return type
- Type inference for the return type is not supported in this initial implementation

### Scope
Expression bodies have access to:
- All function parameters
- `self` in instance methods
- Generic type parameters
- Items visible from the function's scope

### Interaction with Other Features
- **Visibility modifiers**: Work normally (`public func add(...) -> Int = ...`)
- **Static modifier**: Work normally (`static func create() -> Self = ...`)
- **Receiver modifiers**: Work normally (`mutating func increment() -> () = ...`)
- **External functions**: Cannot have expression bodies (same as block bodies)
- **Generics and where clauses**: Fully supported

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Missing return type | "Expression-bodied functions require an explicit return type" |
| External function with expression body | "External functions cannot have a body" |
| Expression type mismatch | "Expected `{expected}`, found `{actual}`" (existing error) |

## Edge Cases

1. **Empty expression**: Not valid - expression must produce a value
2. **Unit return type**: Allowed - `func log(msg: String) -> () = print(msg)`
3. **Closure as body**: Allowed - `func makeAdder(n: Int) -> (Int) -> Int = { x in lang.i64_add(x, n) }`
4. **Nested function calls**: Allowed - `func foo() -> Int = bar(baz())`

## Open Questions (Resolved)

1. **Should return type be required?**
   - **Resolution**: Yes, return type is required. This simplifies implementation and matches the existing pattern where only closures have inferred return types.

2. **Should multi-line expressions be supported?**
   - **Resolution**: Yes, naturally supported since expressions can span multiple lines.

3. **What is the syntax order with where clauses?**
   - **Resolution**: `func name[T](params) -> Type where T: Bound = expression`

4. **Should methods support this syntax?**
   - **Resolution**: Yes, both instance and static methods support expression bodies.

5. **Should protocol default implementations support this?**
   - **Resolution**: Yes, protocols can provide default implementations using expression bodies.
