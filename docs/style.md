# Kestrel Style Guide

This document describes idiomatic patterns and style guidelines for writing Kestrel code.

## Literals

### Integer Literals

Use type annotations on variables instead of explicit integer literal constructors:

```kestrel
// GOOD
let x: Int32 = 42
let max: UInt64 = 1000000

// BAD - verbose and unnecessary
let x = Int32(intLiteral: 42)
let max = UInt64(intLiteral: 1000000)
```

This works because Kestrel infers the literal type from the annotated variable type.

## Types

### Type Operators

Use type operators instead of explicit generic type names:

```kestrel
// GOOD
let name: String? = .None
let numbers: [Int64] = [1, 2, 3]
let ages: [String: Int64] = [:]
func parse(input: String) -> Int64 throws ParseError { ... }

// INSTEAD OF
let name: Optional[String] = .None
let numbers: Array[Int64] = [1, 2, 3]
let ages: Dictionary[String, Int64] = [:]
func parse(input: String) -> Result[Int64, ParseError] { ... }
```

Type operator reference:
- `T?` → `Optional[T]`
- `[T]` → `Array[T]`
- `[K: V]` → `Dictionary[K, V]`
- `T throws E` → `Result[T, E]`

## Error Handling

### Try Operator

Use the `try` operator with `match` for error handling:

```kestrel
// GOOD
let result = try someOperation()
if let .Some(value) = result {
    // use value
} else {
    // handle error
}

// Or use ?? for defaults
let value = try someOperation() ?? defaultValue
```

## Pattern Matching

### If-Let

Use `if let` for unwrapping Optionals:

```kestrel
// GOOD
if let .Some(value) = maybeValue {
    // value is unwrapped here
}

// INSTEAD OF
if maybeValue.isSome() {
    let value = maybeValue.unwrap()
    // use value
}
```

### Guard-Let

Use `guard let` for early exit when unwrapping fails:

```kestrel
// GOOD
func process(maybeValue: Int?) -> Int {
    guard let .Some(value) = maybeValue else {
        return 0
    }
    // value is unwrapped here, no nesting needed
    return value * 2
}

// INSTEAD OF
func process(maybeValue: Int?) -> Int {
    if let .Some(value) = maybeValue {
        return value * 2
    } else {
        return 0
    }
}
```

### While-Let

Use `while let` for iterating until None:

```kestrel
// GOOD
while let .Some(item) = iterator.next() {
    // process item
}

// INSTEAD OF
var done = false
while !done {
    let item = iterator.next()
    if item.isSome() {
        let value = item.unwrap()
        // process value
    } else {
        done = true
    }
}
```

## Ranges

### Range Operators

Use range operators instead of explicit Range construction:

```kestrel
// GOOD
for i in 0..<10 { }      // exclusive upper bound
for i in 0..=9 { }       // inclusive (using ..= operator)

// INSTEAD OF
for i in Range[Int64](0, 10) { }
```

## Control Flow

### For-In Loops

Use `for-in` syntax for iteration:

```kestrel
// GOOD
for elem in collection {
    // process elem
}

// INSTEAD OF
var iter = collection.iter()
while let .Some(elem) = iter.next() {
    // process elem
}
```

## Type Promotion (Future)

Once implemented, these patterns will be preferred:

### Optional Promotion

```kestrel
// Will be preferred once implemented
let x: Int? = 5        // Desugars to Optional.Some(5)
```

### Result Promotion

```kestrel
// Will be preferred once implemented
func compute() -> Int throws Error {
    return 42           // Desugars to Result.Ok(42)
}
```

### Throw Expression

```kestrel
// Will be preferred once implemented
func divide(a: Int, b: Int) -> Int throws Error {
    if b == 0 { throw Error("division by zero") }
    return a / b
}
```
