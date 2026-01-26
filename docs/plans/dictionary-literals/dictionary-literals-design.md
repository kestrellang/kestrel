# Dictionary Literals Design

## Overview

Dictionary literals provide a concise syntax for creating dictionaries inline, similar to array literals. They allow initialization of `Dictionary[K, V]` types (or any type conforming to `ExpressibleByDictionaryLiteral`) from key-value pairs.

## Syntax

```kestrel
// Empty dictionary (type inferred from context)
let empty: [String: Int] = [:]

// Non-empty dictionary
let ages: [String: Int] = ["Alice": 30, "Bob": 25]

// Type inference from context
func process(data: [String: Int]) { }
process(["x": 1, "y": 2])

// Default type when no context
let inferred = ["key": "value"]  // Dictionary[String, String, DefaultHasher]
```

## Semantic Behavior

### Type Inference
- Dictionary literals have type `Ty::Infer` until resolved
- The compiler adds a conformance constraint to `_ExpressibleByDictionaryLiteral`
- `Key` and `Value` associated types are resolved via normalization constraints
- All keys must unify to the same `Key` type
- All values must unify to the same `Value` type

### Protocol Conformance
The type system uses a two-layer protocol design (matching array literals):

```kestrel
// Low-level protocol - compiler calls this directly
@builtin(._ExpressibleByDictionaryLiteral)
protocol _ExpressibleByDictionaryLiteral {
    type Key
    type Value
    init(_dictionaryLiteralPointer: lang.ptr[(Key, Value)], _dictionaryLiteralCount: lang.i64)
}

// User-facing protocol - takes LiteralSlice for convenience
@builtin(.ExpressibleByDictionaryLiteral)
protocol ExpressibleByDictionaryLiteral: _ExpressibleByDictionaryLiteral {
    type Key
    type Value
    init(dictionaryLiteral: LiteralSlice[(Key, Value)])
}

// Default implementation bridges the two
extend ExpressibleByDictionaryLiteral {
    init(_dictionaryLiteralPointer: lang.ptr[(Key, Value)], _dictionaryLiteralCount: lang.i64) {
        self.init(dictionaryLiteral: LiteralSlice(
            pointer: _dictionaryLiteralPointer,
            count: _dictionaryLiteralCount
        ))
    }
}
```

### Default Type
When a dictionary literal has no type context, the compiler uses:
```kestrel
@builtin(.DefaultDictionaryLiteralType)
public type DictionaryLiteralType[K, V] = Dictionary[K, V, DefaultHasher]
```

### Code Generation
1. Evaluate all key-value pairs to create tuple array `[(K, V)]`
2. Allocate stack buffer for the tuple array
3. Get pointer to buffer: `lang.ptr[(Key, Value)]`
4. Get count: number of pairs as `lang.i64`
5. Call `Type._ExpressibleByDictionaryLiteral.init(ptr, count)`

## Parsing Disambiguation

The parser must distinguish:
| Syntax | Interpretation |
|--------|----------------|
| `[]` | Empty array literal |
| `[:]` | Empty dictionary literal |
| `[expr]` | Single-element array |
| `[expr: expr]` | Single-pair dictionary |
| `[e1, e2]` | Multi-element array |
| `[k1: v1, k2: v2]` | Multi-pair dictionary |

**Strategy**: After `[`, use look-ahead:
1. If next token is `]` → empty array
2. If next token is `:` followed by `]` → empty dictionary (`[:]`)
3. Parse first expression
4. If next token is `:` → dictionary (parse value, continue with pairs)
5. Otherwise → array (continue with elements)

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Mixed array/dict syntax | "cannot mix array and dictionary literal syntax" |
| Missing value after `:` | "expected expression after ':' in dictionary literal" |
| Type doesn't conform | "type 'X' does not conform to 'ExpressibleByDictionaryLiteral'" |
| Key type mismatch | "cannot convert 'X' to expected key type 'Y'" |
| Value type mismatch | "cannot convert 'X' to expected value type 'Y'" |
| No type context for `[:]` | "cannot infer type for empty dictionary literal" |

## Edge Cases

1. **Trailing comma**: `["a": 1, "b": 2,]` - allowed (consistent with arrays)
2. **Single pair**: `["key": value]` - valid dictionary with one entry
3. **Nested literals**: `["outer": ["inner": 1]]` - dictionary of dictionaries
4. **Computed keys**: `[computeKey(): value]` - any expression allowed as key
5. **Duplicate keys**: Allowed at compile time (runtime behavior defined by Dictionary)
6. **Generic contexts**: `func f<T: ExpressibleByDictionaryLiteral>(_ d: T)` works with literals

## Open Questions (Resolved)

1. **Q: Should empty dictionary require type annotation?**
   A: No, infer from context like arrays. Error if no context available.

2. **Q: What expressions are allowed as keys?**
   A: Any expression (not restricted to literals).

3. **Q: Default type when no inference context?**
   A: Use `@builtin(.DefaultDictionaryLiteralType)` type alias pointing to `Dictionary[K, V, DefaultHasher]`.

4. **Q: How to parse `[` token ambiguity?**
   A: Look-ahead after first expression to check for `:`.
