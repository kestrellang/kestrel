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

An enum that contains itself by value would have infinite size, so a plain enum
may not be recursive (E429). Mark it `indirect` and the compiler stores each
**recursive case** behind a heap box:

```kestrel
indirect enum List[T] {
    case Nil
    case Cons(head: T, tail: List[T])
}

indirect enum Tree {
    case Leaf
    case Node(Tree, Int64, Tree)
}
```

`indirect` is a contextual keyword, special only in this position.

**Which cases are boxed.** Exactly the cases whose payload reaches the enum by
value — directly (`Tree`), inside a tuple, through a struct field, through
`Optional[Self]`, or through another indirect enum. The whole payload of such a
case lives in one `std.memory.OwnedBox`, so a case costs one allocation however
many fields it has. Cases that don't recurse (`Nil`, `Leaf`) stay inline and
allocate nothing. An `indirect enum` with no recursive case boxes nothing and
gets a warning (E405).

**Using one.** Construction and pattern matching look exactly like any other
enum: `.Cons(head: 1, tail: .Nil)`, `match list { .Cons(h, t) => … }`. Matching
a borrowed value reads the payload in place; a consuming match moves it out
and frees the box; `&mutating` bindings write into the box in place:

```kestrel
extend Tree {
    mutating func insert(_ v: Int64) {
        match self {
            .Leaf => { self = .Node(.Leaf, v, .Leaf); },
            .Node(&mutating l, x, &mutating r) => if v < x { l.insert(v) } else { r.insert(v) }
        }
    }
}
```

**Copying.** Copy semantics come from the fields as usual — the box is never
bit-copied. An indirect enum whose payloads are all copyable is `Copyable`,
and **a copy clones every box**: `let b = a` duplicates the whole list or
tree, which is O(n). Mark the enum `not Copyable` when values should only
move (then `list = .Cons(x, list)` moves the old list in instead of copying
it). A move-only payload makes the enum move-only.

**Limits (0.19).**
- The recursive reference must use the enum's own type parameters:
  `case Nest(Nested[Array[T]])` inside `Nested[T]` is rejected (E407).
- Indirect enums need the standard library's `OwnedBox` (E406 under `--no-std`).
- Dropping a very long chain recurses once per node, so dropping a list of
  about 100,000 nodes can overflow the stack.

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

#### E429: Recursive enum requires `indirect`

```kestrel
enum Tree {
    case Leaf(value: Int64)
    case Node(left: Tree, right: Tree)  // error!
}
```

```
error: recursive enum requires `indirect` [E429]
 = add 'indirect' before the enum declaration to allow recursive cases
```

#### E405: `indirect` has no effect (warning)

```kestrel
indirect enum Shape {    // warning: no case contains `Shape`
    case Circle(Int64)
    case Square(Int64)
}
```

```
warning: `indirect` has no effect: enum 'Shape' has no recursive case [E405]
```

#### E406: Indirect enums require the standard library

Boxing a recursive case uses `std.memory.OwnedBox`, so `indirect enum` is an
error under `--no-std`.

#### E407: Indirect enum contains itself with different type arguments

```kestrel
indirect enum Nested[T] {
    case Flat(T)
    case Nest(Nested[Array[T]])  // error!
}
```

Every level would be a new instantiation of `Nested`; a recursive case must
use the enum's own type parameters unchanged.

#### E427: Duplicate case name

```kestrel
enum Color {
    case Red
    case Red  // error!
}
```

```
error: duplicate enum case 'Red' [E427]
 2 |     case Red
   |     -------- first defined here
 3 |     case Red
   |     ^^^^^^^^ duplicate case defined here
```

#### E428: Duplicate label in case

```kestrel
enum Bad {
    case Foo(x: Int64, x: String)  // error!
}
```

```
error: duplicate label 'x' in case 'Foo' [E428]
```

### Instantiation Errors

Instantiation mistakes are reported by name resolution and the type checker rather than by enum-specific diagnostic codes:

```kestrel
enum Shape {
    case Circle(radius: Float64)
    case Point
}

// Unknown case
let c = Color.Purple;
// error: undefined name 'Color.Purple'

// Missing or wrong associated value label — labels are part of
// the case's signature, so this reads as a different overload
let s = Shape.Circle(5.0);
// error: no matching overload for 'Circle'

// Shorthand without type context
let x = .Red;
// error: implicit member '.Red' not found
// fix: `let x: Color = .Red` or `Color.Red`

// Associated value type mismatch or wrong arity
let t = Shape.Circle(radius: "big");
// error: no matching overload for 'Circle'
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
- Detect recursion and require `indirect` (shared `kestrel_semantics::RecursiveCases` query; MIR boxes the cases `IndirectCases` names)
- Validate associated value types

### Type Inference

- `.Case` shorthand uses bidirectional type checking
- Expected type propagates to enum case expression
- Generic type arguments inferred from associated values
