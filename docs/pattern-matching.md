# Pattern Matching

Pattern matching provides a powerful way to destructure values and control flow based on their shape. Kestrel supports pattern matching in match expressions, if-let, while-let, guard-let, and destructuring bindings.

## Match Expressions

Match expressions evaluate a value (the scrutinee) against a series of patterns, executing the body of the first matching arm.

```kestrel
match value {
    .Red or .Orange or .Yellow => "warm",
    .Green or .Blue => "cool",
    _ => "other"
}
```

Match is an expression - it returns a value. All arms must have compatible types.

### Syntax

```
match scrutinee {
    pattern => body,
    pattern => body,
    ...
}
```

- No `case` keyword before arms
- `=>` separates pattern from body
- Trailing comma after body is optional
- Body can be a single expression or a block

### Guards

Guards add conditions to patterns using `if`:

```kestrel
match number {
    .Some(n) if n > 0 => "positive",
    .Some(n) if n < 0 => "negative",
    .Some(_) => "zero",
    .None => "nothing"
}
```

Guards are checked after the pattern matches. The compiler treats guards conservatively for exhaustiveness - it assumes guards may fail, so a fallback arm is typically required.

#### Guard Evaluation Order

Guards are evaluated top-to-bottom after pattern matching succeeds. With or-patterns, a guard may be evaluated multiple times if earlier alternatives match but the guard fails:

```kestrel
match value {
    .A(x) or .B(x) if check(x) => use(x),
    _ => default()
}
```

If `value` is `.A(1)` and `check(1)` returns false, the pattern `.B(x)` is tried next. If that also matches (which it won't for `.A(1)`), the guard runs again. Side effects in guards are allowed but should be used carefully.

### Or-Patterns

Use `or` to match multiple patterns with the same body:

```kestrel
match color {
    .Red or .Orange or .Yellow => "warm",
    .Green or .Blue or .Purple => "cool"
}
```

Or-patterns can be nested within other patterns:

```kestrel
match expr {
    .Binary(.Add or .Sub, left, right) => handleAddSub(left, right),
    .Binary(.Mul or .Div, left, right) => handleMulDiv(left, right),
    .Unary(.Neg or .Not, operand) => handleUnary(operand),
    _ => other()
}

match opt {
    .Some(1 or 2 or 3) => "small",
    .Some(n) => "large",
    .None => "nothing"
}
```

Rules:
- All alternatives in an or-pattern must bind the same names with the same types
- Or-patterns can be nested at any level

```kestrel
// OK: both bind `value` as Int
.Some(value) or .Other(value) => use(value)

// OK: nested or-pattern, no bindings
.Some(1 or 2 or 3) => "small"

// ERROR: inconsistent bindings
.Some(value) or .None => ...  // .None doesn't bind `value`
```

## Pattern Types

### Wildcard Pattern

Matches any value, binds nothing:

```kestrel
_ => handleDefault()
```

### Binding Pattern

Matches any value, binds to an identifier:

```kestrel
value => use(value)           // immutable binding
var value => { value = 1 }    // mutable binding
```

### Literal Pattern

Matches an exact value:

```kestrel
42 => "the answer",
"hello" => "greeting",
true => "affirmative",
```

### Range Pattern

Matches a range of values using `..` (exclusive end) or `..=` (inclusive end):

```kestrel
match c {
    'a'..='z' => "lowercase",
    'A'..='Z' => "uppercase",
    '0'..='9' => "digit",
    _ => "other"
}

match score {
    0..=59 => "F",
    60..=69 => "D",
    70..=79 => "C",
    80..=89 => "B",
    90..=100 => "A",
    _ => "invalid"
}
```

Range patterns work with integers and characters. Both bounds must be literals of the same type.

### Tuple Pattern

Destructures tuples:

```kestrel
(x, y) => x + y,
(a, (b, c)) => a + b + c,    // nested
(first, _) => first,          // ignore elements with _
```

### Rest Pattern

Use `..` to ignore remaining elements in a tuple:

```kestrel
(first, ..) => first,              // bind first, ignore rest
(first, second, ..) => first + second,
(.., last) => last,                // ignore all but last
(first, .., last) => (first, last) // bind first and last only
```

Rest patterns can appear at most once in a tuple pattern. They match zero or more elements.

### Enum Variant Pattern

Matches enum cases and destructures associated values:

```kestrel
// Simple cases (no associated values)
.None => handleNone()

// With associated values - shorthand (binding name = label)
.Some(value) => use(value)
.Circle(radius) => pi * radius * radius

// With associated values - explicit rename
.Some(value: v) => use(v)
.Rectangle(width: w, height: h) => w * h

// Mixed shorthand and explicit
.Rectangle(width, height: h) => width * h

// Wildcards for ignored values
.Circle(_) => "it's a circle"
.Rectangle(_, height) => height
```

### Associated Value Rules

| Syntax | Meaning |
|--------|---------|
| `.Case(label)` | Shorthand: binds to `label` |
| `.Case(label: binding)` | Explicit: binds to `binding` |
| `.Case(_)` | Wildcard: ignores value |
| `.Case(var label)` | Mutable shorthand binding |
| `.Case(label: var binding)` | Mutable explicit binding |

Labels are required and must match the enum case declaration (consistent with instantiation syntax).

### Struct Pattern

Destructures structs using curly brace syntax with named fields:

```kestrel
struct Point { x: Int, y: Int }

match p {
    Point { x: 0, y } => "on y-axis",
    Point { x, y: 0 } => "on x-axis",
    Point { x, y } if x == y => "diagonal",
    Point { .. } => "elsewhere"
}
```

Use `..` to ignore remaining fields:

```kestrel
struct Person { name: String, age: Int, email: String }

match person {
    Person { name, .. } => greet(name),
}
```

#### Struct Pattern Syntax

| Syntax | Meaning |
|--------|---------|
| `Type { field }` | Shorthand: binds field value to `field` |
| `Type { field: binding }` | Explicit: binds field value to `binding` |
| `Type { field: pattern }` | Nested: matches field against pattern |
| `Type { field, .. }` | Partial: ignores remaining fields |
| `Type { .. }` | Matches any instance, binds nothing |

All mentioned fields must be visible (public or in the same module). Use `..` to acknowledge fields you cannot or do not want to name.

#### Visibility in Struct Patterns

When matching structs with private fields from outside the defining module, you must use `..` to ignore inaccessible fields:

```kestrel
// In module geometry
pub struct Point {
    pub x: Int,
    y: Int,      // private
}

// In another module
match p {
    Point { x, .. } => use(x),     // OK: ignores private field y
    Point { x, y } => ...,          // ERROR: field `y` is private
}
```

This design ensures that adding private fields to a struct doesn't break external pattern matches that use `..`.

### Array Pattern

Destructures arrays and slices:

```kestrel
match arr {
    [] => "empty",
    [only] => "single element",
    [first, second] => "exactly two",
    [first, ..] => "at least one",
    [.., last] => "at least one (get last)",
    [first, .., last] => "at least two",
    [first, ..rest] => "first and rest as slice",
    [1, 2, ..] => "starts with 1, 2"
}
```

#### Array Pattern Rules

- `[]` matches an empty array
- `[a, b, c]` matches exactly 3 elements
- `..` matches zero or more elements (ignores them)
- `..name` binds the matched elements to a slice
- At most one `..` or `..name` per array pattern
- Fixed patterns determine minimum length requirements:
  - `[first, ..]` requires at least 1 element
  - `[first, .., last]` requires at least 2 elements

### Nested Patterns

Patterns can be nested arbitrarily:

```kestrel
match result {
    .Ok(.Some(value)) => use(value),
    .Ok(.None) => handleEmpty(),
    .Err(error) => handleError(error)
}

match data {
    Point { x, y: 0 } => "on x-axis at {x}",
    Person { name, address: Address { city, .. }, .. } => "{name} lives in {city}"
}
```

### Binding with Subpattern (@-patterns)

Bind the entire matched value while also matching against a subpattern using `@`:

```kestrel
match list {
    node @ .Cons(head, _) => {
        // node is the whole Cons value, head is just the first element
        process(node, head)
    },
    .Nil => handleEmpty()
}

match opt {
    some @ .Some(_) => returnAsIs(some),
    .None => default()
}
```

This is useful when you need both the whole value and its destructured parts.

#### @-pattern Precedence

The `or` operator has lower precedence than `@`. Use parentheses when combining them:

```kestrel
// Correct: x binds to the whole matched value
x @ (.Some(_) or .None) => use(x)

// Incorrect: parsed as (x @ .Some(_)) or .None
// ERROR: x is not bound in .None alternative
x @ .Some(_) or .None => use(x)
```

## If-Let Expressions

Use `if let` for conditional pattern matching when you only care about one pattern:

```kestrel
if let .Some(value) = optional {
    use(value)
}

if let .Some(value) = optional {
    use(value)
} else {
    handleNone()
}
```

The bindings are only in scope within the then-branch.

### If-Let Chains

Combine multiple pattern matches and boolean conditions with commas:

```kestrel
if let .Some(x) = optX, let .Some(y) = optY {
    use(x, y)
}

if let .Some(x) = optX, x > 0 {
    usePositive(x)
}

if let .Some(x) = optX, let .Some(y) = optY, x > y {
    useLarger(x)
}
```

Conditions are evaluated left-to-right. If any pattern fails to match or any boolean condition is false, the else branch (if present) is taken. Bindings from earlier conditions are in scope for later conditions.

## While-Let Expressions

Use `while let` to loop while a pattern matches:

```kestrel
while let .Some(item) = iterator.next() {
    process(item)
}
```

The loop exits when the pattern fails to match.

## Guard-Let Statements

Use `guard let` for early exit when a pattern doesn't match:

```kestrel
fn process(opt: Option[Int]) -> Int {
    guard let .Some(value) = opt else {
        return 0
    }
    // value is in scope for rest of function
    return value * 2
}
```

The `else` block must diverge (return, break, continue, or panic). The bindings are in scope after the guard statement for the rest of the enclosing block.

## Destructuring Bindings

`let` and `var` statements accept full patterns:

```kestrel
// Tuple destructuring
let (x, y) = (1, 2)
let (a, (b, c)) = (1, (2, 3))

// With wildcards
let (x, _) = (1, 2)

// Mutable bindings
var (x, y) = (1, 2)           // both mutable

// Mixed mutability
let (var x, y) = (1, 2)       // x mutable, y immutable
```

### Refutability

Patterns in `let`/`var` must be irrefutable (always match). Use `if let` or `guard let` for refutable patterns:

```kestrel
// ERROR: refutable pattern in let binding
let .Some(value) = optional

// OK: use if let for refutable patterns
if let .Some(value) = optional {
    use(value)
}

// OK: use guard let for early exit
guard let .Some(value) = optional else {
    return
}
```

## Exhaustiveness

Match expressions must be exhaustive - all possible values must be covered.

| Type | Exhaustive When |
|------|-----------------|
| Enum | All cases covered |
| Bool | `true` and `false` covered |
| Tuple | Each element exhaustive |
| Struct | All fields exhaustive (with `..` for ignored fields) |
| Array | Requires `_` or `[..]` wildcard |
| Char | Requires `_` wildcard |
| Int/String | Requires `_` wildcard |

Guards are treated conservatively - the compiler assumes they may fail:

```kestrel
// ERROR: non-exhaustive (guard might fail)
match opt {
    .Some(n) if n > 0 => "positive",
    .None => "nothing"
}

// OK: fallback covers when guard fails
match opt {
    .Some(n) if n > 0 => "positive",
    .Some(_) => "non-positive",
    .None => "nothing"
}
```

## Errors

| Code | Description |
|------|-------------|
| E0501 | Non-exhaustive match - missing pattern cases |
| E0502 | Refutable pattern in irrefutable context (use `if let` or `guard let`) |
| E0503 | Pattern type mismatch |
| E0504 | Inconsistent bindings in or-pattern |
| E0505 | Unknown enum case in pattern |
| E0506 | Missing associated value label |
| E0507 | Wrong associated value label |
| W0508 | Unreachable match arm |
| W0509 | Irrefutable pattern in `if let` (use plain `let` instead) |
| E0510 | Duplicate binding in pattern |
| E0511 | Inconsistent mutability in or-pattern |
| E0512 | Guard condition must be Bool |
| E0513 | Wrong number of elements in tuple pattern |
| E0514 | Wrong number of associated values in enum pattern |
| E0515 | Guard-let else block must diverge |
| W0516 | Unused binding in pattern (use `_` or `_name` prefix) |
| E0517 | Empty match on non-Never type |
| W0518 | Binding name matches enum case (did you mean `.Case`?) |
| E0519 | Invalid @-pattern (left side must be a binding) |
| E0520 | Multiple rest patterns in tuple |
| E0521 | Invalid range pattern bounds (lower > upper) |
| E0522 | Range pattern type mismatch |
| E0523 | Range pattern on unsupported type (only Int and Char) |
| E0524 | Float literal in pattern (use guard instead) |
| E0525 | Private field in struct pattern |
| E0526 | Unknown field in struct pattern |
| E0527 | Missing fields in struct pattern (use `..` to ignore) |
| E0528 | Multiple rest patterns in array |
| W0529 | Overlapping range patterns |
| W0530 | Duplicate pattern |

## Grammar

```
match_expr := "match" expr "{" match_arm* "}"
match_arm := or_pattern ("if" expr)? "=>" expr ","?

or_pattern := pattern ("or" pattern)*

pattern :=
    | "_"
    | ".."
    | ".." IDENT
    | IDENT
    | "var" IDENT
    | IDENT "@" pattern
    | literal_pattern
    | range_pattern
    | tuple_pattern
    | array_pattern
    | struct_pattern
    | enum_pattern
    | "(" or_pattern ")"

literal_pattern := INTEGER | STRING | CHAR | "true" | "false"

range_pattern := 
    | range_bound ".." range_bound
    | range_bound "..=" range_bound

range_bound := INTEGER | CHAR

tuple_pattern := "(" tuple_elements ")"
tuple_elements := 
    | pattern ("," pattern)* ("," "..")?
    | ".." ("," pattern)+
    | pattern ("," pattern)* "," ".." "," pattern ("," pattern)*

array_pattern := "[" array_elements? "]"
array_elements :=
    | pattern ("," pattern)* ("," (".." IDENT?))?
    | (".." IDENT?) ("," pattern)+
    | pattern ("," pattern)* "," (".." IDENT?) "," pattern ("," pattern)*

struct_pattern := IDENT "{" struct_field_patterns? "}"
struct_field_patterns := 
    | struct_field_pattern ("," struct_field_pattern)* ("," "..")?
    | ".."
struct_field_pattern :=
    | IDENT
    | "var" IDENT
    | IDENT ":" or_pattern

enum_pattern := "." IDENT ("(" assoc_bindings ")")?
assoc_bindings := assoc_binding ("," assoc_binding)*
assoc_binding :=
    | "_"
    | IDENT
    | "var" IDENT
    | IDENT ":" or_pattern

if_let_expr := "if" condition_list block ("else" (block | if_expr))?
condition_list := condition ("," condition)*
condition := "let" pattern "=" expr | expr

while_let_expr := "while" "let" pattern "=" expr block
guard_let_stmt := "guard" "let" pattern "=" expr "else" block

let_stmt := "let" pattern (":" type)? "=" expr
var_stmt := "var" pattern (":" type)? "=" expr
```

### Grammar Notes

**If-let condition parsing:** The comma (`,`) separates conditions in if-let chains. Each condition is either `let pattern = expr` or a boolean expression. The parser distinguishes them by the leading `let` keyword. Comma expressions are not supported in Kestrel, so there is no ambiguity.

**@-pattern precedence:** The `or` operator binds looser than `@`. The pattern `x @ .A or .B` parses as `(x @ .A) or .B`, not `x @ (.A or .B)`. Use parentheses for the latter.

## Future Considerations

The following features are not part of the initial design but may be added later:

### For-Loop Patterns

Destructuring in for loops (requires for-loop implementation first):

```kestrel
for (key, value) in map {
    use(key, value)
}

for .Some(item) in optionalItems {
    process(item)
}
```

### Bounded Integer Exhaustiveness

For fixed-width integer types like `U8`, `I8`, etc., the compiler could track range coverage and determine exhaustiveness without requiring a wildcard:

```kestrel
// Potentially exhaustive in future (U8 has 256 values)
match byte {
    0..=127 => "low",
    128..=255 => "high"
}
```

The exhaustiveness checker architecture supports adding this by making coverage analysis type-driven. Currently all integer types require a wildcard for simplicity.

### Constant Patterns

When constants are added to the language, they could be usable in patterns:

```kestrel
const MAX_SIZE: Int = 100

match n {
    MAX_SIZE => "at maximum",
    _ => "not at maximum"
}
```

This would require distinguishing constant references from binding patterns, likely through qualification (`module.CONST`) or a sigil.
