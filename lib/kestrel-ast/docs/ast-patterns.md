# AST Patterns

Patterns are stored in an `Arena<AstPat>` inside `AstBody`, addressed by `PatId`.

Patterns appear in `let`/`var` bindings, `match` arms, `for` loops, `if let`/`while let`/`guard let` conditions, and closure parameters.

## `AstPat` Variants

### `Wildcard`

Matches anything, binds nothing: `_`

| Field | Type | Description |
|-------|------|-------------|
| `span` | `Span` | Source location |

### `Binding`

Binds a value to a name: `x`, `var y`

| Field | Type | Description |
|-------|------|-------------|
| `is_mut` | `bool` | `true` for `var` bindings |
| `name` | `String` | Variable name |
| `span` | `Span` | Source location |

Examples:
```
let x = 42;           // Binding { is_mut: false, name: "x" }
var count = 0;        // Binding { is_mut: true, name: "count" }
```

### `Tuple`

Destructures a tuple: `(a, b, c)`

| Field | Type | Description |
|-------|------|-------------|
| `elements` | `Vec<PatId>` | Sub-patterns for each element |
| `span` | `Span` | Source location |

Example:
```
let (x, y) = pair;    // Tuple([Binding("x"), Binding("y")])
```

### `Literal`

Matches a literal value: `42`, `"hello"`, `true`

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `LitPatKind` | The literal value |
| `span` | `Span` | Source location |

See [LitPatKind](#litpatkind) below.

### `Range`

Matches a range of values: `1..=10`, `0..<n`

| Field | Type | Description |
|-------|------|-------------|
| `start` | `Option<LitPatKind>` | Range start (None for open start) |
| `end` | `Option<LitPatKind>` | Range end (None for open end) |
| `inclusive` | `bool` | `true` for `..=`, `false` for `..<` |
| `span` | `Span` | Source location |

### `Enum`

Matches an enum case: `.None`, `.Some(value)`, `.Pair(first: a, second: b)`

| Field | Type | Description |
|-------|------|-------------|
| `case_name` | `String` | Enum case name |
| `args` | `Vec<EnumPatArg>` | Destructured arguments (empty if none) |
| `span` | `Span` | Source location |

Examples:
```
match opt {
    .None => ...,                  // Enum { case_name: "None", args: [] }
    .Some(x) => ...,              // Enum { case_name: "Some", args: [EnumPatArg { label: None, pattern: Binding("x") }] }
    .Pair(first: a, second: b) => ...,  // with labeled args
}
```

### `Struct`

Matches a struct by fields: `Point { x, y: b, .. }`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Struct name |
| `fields` | `Vec<StructPatField>` | Field patterns |
| `has_rest` | `bool` | `true` if `..` is present (ignore remaining fields) |
| `span` | `Span` | Source location |

Examples:
```
Point { x, y }         // fields bind to same name: StructPatField { field_name: "x", pattern: None }
Point { x: a, y: b }   // explicit binding: StructPatField { field_name: "x", pattern: Some(Binding("a")) }
Point { x, .. }        // has_rest: true
```

### `Array`

Matches an array with optional rest element: `[a, b]`, `[first, ..rest, last]`

| Field | Type | Description |
|-------|------|-------------|
| `prefix` | `Vec<PatId>` | Patterns before the rest element |
| `rest` | `Option<Option<String>>` | `None` = no rest, `Some(None)` = anonymous `..`, `Some(Some(name))` = named `..name` |
| `suffix` | `Vec<PatId>` | Patterns after the rest element |
| `span` | `Span` | Source location |

Examples:
```
[a, b]               // prefix: [Binding("a"), Binding("b")], rest: None, suffix: []
[first, .., last]    // prefix: [Binding("first")], rest: Some(None), suffix: [Binding("last")]
[head, ..tail]       // prefix: [Binding("head")], rest: Some(Some("tail")), suffix: []
```

### `At`

Binds a name while also matching a sub-pattern: `name @ pattern`

| Field | Type | Description |
|-------|------|-------------|
| `is_mut` | `bool` | `true` for `var` binding |
| `name` | `String` | Variable name |
| `subpattern` | `PatId` | Inner pattern to match |
| `span` | `Span` | Source location |

Example:
```
x @ .Some(_)          // At { name: "x", subpattern: Enum("Some", [Wildcard]) }
```

### `Or`

Matches any of several alternatives: `pattern1 | pattern2`

| Field | Type | Description |
|-------|------|-------------|
| `alternatives` | `Vec<PatId>` | Two or more alternative patterns |
| `span` | `Span` | Source location |

Example:
```
.A | .B | .C => ...   // Or([Enum("A"), Enum("B"), Enum("C")])
```

### `Rest`

Rest pattern inside array or struct patterns: `..`

| Field | Type | Description |
|-------|------|-------------|
| `span` | `Span` | Source location |

### `Error`

Produced for malformed pattern CST nodes. Allows lowering to continue without panicking.

| Field | Type | Description |
|-------|------|-------------|
| `span` | `Span` | Source location |

## Supporting Types

### `LitPatKind`

Literal values used in `Literal` and `Range` patterns:

| Variant | Example |
|---------|---------|
| `Integer(String)` | `42` |
| `Float(String)` | `3.14` |
| `String(String)` | `"hello"` |
| `Bool(bool)` | `true` |
| `Char(String)` | `'a'` |

Values are stored as source text strings (not parsed numbers) to preserve formatting.

### `EnumPatArg`

A single argument in an enum pattern.

| Field | Type | Description |
|-------|------|-------------|
| `label` | `Option<String>` | Argument label, if present |
| `pattern` | `PatId` | Pattern for this argument |

### `StructPatField`

A single field in a struct pattern.

| Field | Type | Description |
|-------|------|-------------|
| `field_name` | `String` | Field name |
| `pattern` | `Option<PatId>` | Explicit sub-pattern, or `None` for shorthand (`{ x }` binds `x` to itself) |
