# Enums

Enums (enumerated types) represent a type with a fixed set of possible values called cases. Each case can optionally carry associated values.

## Declaration Syntax

The `case` keyword is required for each variant.

### Simple Enums

```kestrel
enum Color {
    case Red
    case Green
    case Blue
}
```

### Enums with Associated Values

Associated values can use either labeled or unlabeled (positional) syntax.

#### Labeled Parameters

Labels are required at both declaration and instantiation.

```kestrel
enum Shape {
    case Circle(radius: Float)
    case Rectangle(width: Float, height: Float)
    case Point
}
```

#### Unlabeled (Positional) Parameters

Parameters can be declared without labels for more concise syntax:

```kestrel
enum Option[T] {
    case Some(T)
    case None
}

enum Either[A, B] {
    case Left(A)
    case Right(B)
}
```

Instantiation uses positional arguments:

```kestrel
let opt = Option.Some(42)
let either = Either.Left("error")
```

Pattern matching uses positional bindings:

```kestrel
match opt {
    .Some(x) => print(x)
    .None => print("nothing")
}
```

**Note**: Internally, the compiler generates synthetic parameter names `_0`, `_1`, etc. for unlabeled parameters. These are implementation details and not accessible in user code.

### Generic Enums

```kestrel
enum Option[T] {
    case Some(value: T)
    case None
}

enum Result[T, E] {
    case Ok(value: T)
    case Error(error: E)
}
```

### Recursive Enums

Recursive enums require the `indirect` keyword before `enum`. This is a contextual keyword (only special in this position).

```kestrel
indirect enum Tree[T] {
    case Leaf(value: T)
    case Node(left: Tree[T], right: Tree[T])
}

indirect enum List[T] {
    case Cons(head: T, tail: List[T])
    case Empty
}
```

The `indirect` keyword tells the compiler to use indirection (heap allocation) for recursive references, preventing infinite-size types.

## Instantiation

### Full Path Syntax

```kestrel
let color = Color.Red
let shape = Shape.Circle(radius: 5.0)
let opt = Option.Some(value: 42)
let tree = Tree.Leaf(value: "hello")
```

### Shorthand Syntax

When the enum type can be inferred from context, use `.Case` shorthand:

```kestrel
// Type annotation
let color: Color = .Red
let shape: Shape = .Circle(radius: 5.0)

// Function arguments
func draw(shape: Shape) { ... }
draw(.Rectangle(width: 10.0, height: 20.0))

// Return statements
func defaultColor() -> Color {
    .Blue
}

// Assignment to typed variable
var status: Status = .Pending
status = .Active
```

### Instantiation Rules

| Rule | Valid | Invalid |
|------|-------|---------|
| Labels required | `.Circle(radius: 5.0)` | `.Circle(5.0)` |
| Empty parens are allowed | `.None` or `.None()` | N/A - both valid |
| Shorthand needs type context | `let c: Color = .Red` | `let c = .Red` |

Note: For valueless cases, both `Color.Red` and `Color.Red()` are valid - empty parens are allowed.

## Errors

### Declaration Errors

#### E0404: Recursive enum requires `indirect`

```kestrel
enum Tree {
    case Leaf(value: Int)
    case Node(left: Tree, right: Tree)  // error!
}
```

```
error[E0404]: recursive enum requires `indirect`
  --> main.ks:1:1
   |
 1 | enum Tree {
   | ^^^^^^^^^ recursive enum
 2 |     case Leaf(value: Int)
 3 |     case Node(left: Tree, right: Tree)
   |                     ----         ---- recursive references
   |
   = help: add `indirect` before `enum`
```

#### E0405: Duplicate case name

```kestrel
enum Color {
    case Red
    case Red  // error!
}
```

```
error[E0405]: duplicate enum case `Red`
  --> main.ks:3:5
   |
 2 |     case Red
   |          --- first definition
 3 |     case Red
   |          ^^^ duplicate case
```

#### E0406: Duplicate label in case

```kestrel
enum Bad {
    case Foo(x: Int, x: String)  // error!
}
```

```
error[E0406]: duplicate label `x` in enum case
  --> main.ks:2:22
   |
 2 |     case Foo(x: Int, x: String)
   |              -       ^ duplicate label
   |              |
   |              first use of `x`
```

### Instantiation Errors

#### E0401: Unknown enum case

```kestrel
let c = Color.Purple  // error!
```

```
error[E0401]: unknown enum case `Purple`
  --> main.ks:1:15
   |
 1 | let c = Color.Purple
   |               ^^^^^^ `Color` has no case `Purple`
   |
   = help: available cases: Red, Green, Blue
```

#### E0402: Missing associated value label

```kestrel
let s = Shape.Circle(5.0)  // error!
```

```
error[E0402]: missing associated value label
  --> main.ks:1:22
   |
 1 | let s = Shape.Circle(5.0)
   |                      ^^^ expected label `radius:`
   |
   = help: use `Shape.Circle(radius: 5.0)`
```

#### E0402: Wrong associated value label

```kestrel
let s = Shape.Circle(r: 5.0)  // error!
```

```
error[E0402]: wrong associated value label
  --> main.ks:1:22
   |
 1 | let s = Shape.Circle(r: 5.0)
   |                      ^^ expected `radius:`, found `r:`
```

#### E0403: Cannot infer enum type for shorthand

```kestrel
let x = .Red  // error!
```

```
error[E0403]: cannot infer enum type for shorthand
  --> main.ks:1:9
   |
 1 | let x = .Red
   |         ^^^^ type annotation needed
   |
   = help: use `let x: Color = .Red` or `Color.Red`
```

#### E0407: Associated value type mismatch

```kestrel
let s = Shape.Circle(radius: "big")  // error!
```

```
error[E0407]: mismatched types in associated value
  --> main.ks:1:30
   |
 1 | let s = Shape.Circle(radius: "big")
   |                              ^^^^^ expected `Float`, found `String`
```

#### E0408: Wrong arity for case

```kestrel
let s = Shape.Rectangle(width: 5.0)  // error! missing height
```

```
error[E0408]: wrong number of associated values
  --> main.ks:1:9
   |
 1 | let s = Shape.Rectangle(width: 5.0)
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected 2 values, found 1
   |
   = help: missing `height: Float`
```

## Type of Enum Values

An enum case instantiation has the type of the enum itself, not a distinct type per case:

```kestrel
let a = Color.Red      // type: Color
let b = Color.Blue     // type: Color
let c = Option.None    // type: Option[???] - needs context

let d: Option[Int] = .None  // type: Option[Int]
let e = Option[Int].None    // type: Option[Int] (explicit)
```

## Enum Methods

Enums can have methods like structs. Both instance and static methods are supported, and enums can conform to protocols.

```kestrel
enum Color {
    case Red
    case Green
    case Blue

    func isWarm() -> Bool {
        // requires pattern matching
    }

    static func default() -> Color {
        .Red
    }
}
```

## Implementation Notes

### Parser

- `indirect` is a contextual keyword, valid as identifier elsewhere
- `case` keyword required before each variant
- Associated values use labeled tuple-like syntax

### Semantic Analysis

- `EnumSymbol` with `CaseSymbol` children
- Cases have `CallableBehavior` for associated values
- Detect recursion and require `indirect`
- Validate associated value types

### Type Inference

- `.Case` shorthand uses bidirectional type checking
- Expected type propagates to enum case expression
- Generic type arguments inferred from associated values
