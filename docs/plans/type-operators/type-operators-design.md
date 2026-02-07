# Type Operators Design

## Overview

Type operators provide syntactic sugar for common generic type patterns. Instead of writing `Optional[Int]`, users can write `Int?`. These operators desugar to type aliases defined in the standard library, keeping the compiler simple while providing ergonomic syntax.

## Motivation

Common type patterns like optionals, arrays, dictionaries, and results are verbose to write repeatedly:

```kestrel
// Verbose
func parse(input: String) -> Result[Ast, ParseError]
func find(id: Int) -> Optional[User]
func getScores() -> Dictionary[String, Array[Int]]

// With type operators
func parse(input: String) -> Ast throws ParseError
func find(id: Int) -> User?
func getScores() -> [String: [Int]]
```

## Syntax

### Optional: `T?`

```kestrel
Int?              // Optional[Int]
String?           // Optional[String]
User??            // Optional[Optional[User]]
```

### Array: `[T]`

```kestrel
[Int]             // Array[Int]
[[String]]        // Array[Array[String]]
[(Int, Bool)]     // Array[(Int, Bool)]
```

### Dictionary: `[K: V]`

```kestrel
[String: Int]           // Dictionary[String, Int]
[String: [Int]]         // Dictionary[String, Array[Int]]
[[String]: Int]         // Dictionary[Array[String], Int]
```

### Result: `T throws E`

```kestrel
Int throws ParseError           // Result[Int, ParseError]
String throws NetworkError      // Result[String, NetworkError]
User throws (NotFound | Forbidden)  // Result with union error type (future)
```

## Semantic Behavior

### Desugaring via Type Aliases

Each type operator desugars to a type alias in the standard library:

```kestrel
// In std/result/optional.ks
@builtin(.OptionalTypeOperator)
public type OptionalTypeOperator[T] = Optional[T]

// In std/collections/array.ks
@builtin(.ArrayTypeOperator)
public type ArrayTypeOperator[T] = Array[T]

// In std/collections/dictionary.ks
@builtin(.DictionaryTypeOperator)
public type DictionaryTypeOperator[K, V] = Dictionary[K, V]

// In std/result/result.ks
@builtin(.ResultTypeOperator)
public type ResultTypeOperator[T, E] = Result[T, E]
```

The `@builtin` attribute allows the compiler to locate these type aliases without hardcoding paths.

### Resolution Flow

1. Parser emits syntax node (e.g., `TyOptional`, `TyArray`, `TyDictionary`, `TyResult`)
2. Type resolver finds the corresponding `@builtin` type alias
3. Type alias is instantiated with the provided type arguments
4. Result is the underlying type (e.g., `Optional[T]`)

### Precedence

| Operator | Precedence | Associativity |
|----------|------------|---------------|
| `?` | Highest | Postfix |
| `[T]`, `[K: V]` | High | Prefix (bracketed) |
| `throws` | Lowest | Infix |

This means:
- `Int throws Error?` parses as `(Int throws Error)?` → `Optional[Result[Int, Error]]`
- `[Int]?` parses as `([Int])?` → `Optional[Array[Int]]`
- `[Int?]` parses as `[Int?]` → `Array[Optional[Int]]`

### Composability

All type operators compose naturally:

```kestrel
// Optional array
[Int]?                    // Optional[Array[Int]]

// Array of optionals
[Int?]                    // Array[Optional[Int]]

// Optional dictionary
[String: Int]?            // Optional[Dictionary[String, Int]]

// Dictionary with optional values
[String: Int?]            // Dictionary[String, Optional[Int]]

// Optional result
Int throws Error?         // Optional[Result[Int, Error]]

// Result with optional error (rare, needs explicit parens)
Int throws (Error?)       // Result[Int, Optional[Error]]

// Array of results
[Int throws Error]        // Array[Result[Int, Error]]

// Result returning array
[Int] throws Error        // Result[Array[Int], Error]

// Complex composition
[String: Int throws ParseError]?  // Optional[Dictionary[String, Result[Int, ParseError]]]
```

## Implementation Changes

### Remove Built-in Array Type

Currently `TyKind::Array(Box<Ty>)` is a primitive type kind. This will be removed:

- Remove `TyKind::Array` variant from `kestrel-semantic-tree/src/ty/kind.rs`
- Add `Array[T]` struct to standard library
- Update array literal expressions to construct `Array[T]`
- Update codegen to handle `Array` as a regular generic struct

### New Syntax Nodes

Add to `SyntaxKind`:
- `TyDictionary` - for `[K: V]` syntax
- `TyResult` - for `T throws E` syntax

(Note: `TyOptional` and `TyArray` already exist)

### New Token

Add `Throws` keyword token to lexer.

### Type Resolver Updates

Handle new syntax nodes by looking up `@builtin` type aliases:
- `TyOptional` → find `@builtin(.OptionalTypeOperator)`, apply `[T]`
- `TyArray` → find `@builtin(.ArrayTypeOperator)`, apply `[T]`
- `TyDictionary` → find `@builtin(.DictionaryTypeOperator)`, apply `[K, V]`
- `TyResult` → find `@builtin(.ResultTypeOperator)`, apply `[T, E]`

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| `@builtin(.OptionalTypeOperator)` not found | "Optional type operator not defined. Is the standard library imported?" |
| `@builtin(.ArrayTypeOperator)` not found | "Array type operator not defined. Is the standard library imported?" |
| `@builtin(.DictionaryTypeOperator)` not found | "Dictionary type operator not defined. Is the standard library imported?" |
| `@builtin(.ResultTypeOperator)` not found | "Result type operator not defined. Is the standard library imported?" |
| `[:]` (empty dictionary type) | "Dictionary type requires key and value types" |
| `throws` without error type | "Expected error type after 'throws'" |
| `throws` at start of type | "Expected type before 'throws'" |

## Edge Cases

### Empty Brackets
- `[]` - Parse error: "Expected type in array brackets"
- `[:]` - Parse error: "Dictionary type requires key and value types"

### Whitespace in Dictionary
- `[K:V]` - Valid
- `[K: V]` - Valid
- `[K : V]` - Valid
- `[ K : V ]` - Valid

### Nested Results
- `Int throws E throws F` - Parse error: "Unexpected 'throws'" (only one throws per type expression level)

### Chained Optional
- `Int??` - Valid: `Optional[Optional[Int]]`
- `Int???` - Valid: `Optional[Optional[Optional[Int]]]`

## Open Questions (Resolved)

### Q: Should type operators use `@builtin` attributes?
**A:** Yes, for consistency with literal defaults and to allow stdlib reorganization without compiler changes.

### Q: What should dictionary type operator be named?
**A:** `DictionaryTypeOperator` to match the stdlib `Dictionary` type name.

### Q: What precedence should `throws` have relative to `?`?
**A:** `throws` binds tighter than `?`, so `Int throws Error?` = `Optional[Result[Int, Error]]`. This optimizes for the common case of "optional result".

### Q: Should built-in `TyKind::Array` be kept?
**A:** No, remove it entirely. Arrays become regular generic structs for uniformity.

## Future Considerations

### Union Types in throws
```kestrel
Int throws (ParseError | NetworkError)  // Future: union error types
```

### Shorthand for common patterns
```kestrel
Int!        // Future: non-optional assertion? Or Result shorthand?
```
