# Kestrel Syntax Guide

This guide covers the essential syntax of the Kestrel programming language.

> **Note**: Features marked as *(Future)* are planned but not yet fully implemented in the parser/compiler.

## Variables and Constants

```kestrel
// Immutable binding (default)
let x: Int = 42
let message = "Hello" // Type inferred

// Mutable variable
var count: Int = 0
count = count + 1

// Type Aliases
type ID = String
type Handler = (Int) -> Bool
```

## Functions

```kestrel
// Basic function
func add(a: Int, b: Int) -> Int {
    a + b
}

// Function with generics
func identity[T](value: T) -> T {
    value
}

// Labeled arguments (External parameter names)
func move(to point: Point, duration seconds: Float64) {
    // ...
}
// Called as: move(to: Point(0, 0), duration: 1.5)

// Function with no external label (using underscore) - (Future)
// func split(_ string: String, by separator: Char) { ... }
```

## Structs

```kestrel
// Basic struct
struct Point {
    var x: Int
    var y: Int
}

// Generic struct
struct Pair[T, U] {
    let first: T
    let second: U
}

// Methods and initializers
struct Circle {
    let radius: Float64;
    
    // Initializer
    init(radius: Float64) {
        self.radius = radius;
    }
    
    // Deinitializer (RAII)
    deinit {
        // Cleanup resources
    }
    
    // Method - NOTE: Unlike Rust, methods do NOT take `self` as a parameter.
    // `self` is implicitly available inside the method body.
    func area() -> Float64 {
        3.14159 * self.radius * self.radius
    }
    
    // WRONG (Rust-style) - Do NOT write methods like this:
    // func area(self) -> Float64 { ... }
    // func area(&self) -> Float64 { ... }
    
    // Mutating method - can modify self
    mutating func scale(by factor: Float64) {
        self.radius = self.radius * factor;
    }
    
    // Static method - no self available
    static func unit() -> Circle {
        Circle(radius: 1.0)
    }
}
```

## Enums

```kestrel
// Simple enum
enum Direction {
    case North
    case South
    case East
    case West
}

// Enum with associated values (Sum Type)
enum Shape {
    case Circle(radius: Float64)
    case Rectangle(width: Float64, height: Float64)
    case Point // No associated value
}

// Recursive enum (must be marked indirect) - (Future)
// enum List[T] {
//     case Cons(T, indirect List[T])
//     case Nil
// }

// Implicit Member Access
let d: Direction = .North
```

## Control Flow

```kestrel
// If-else
if x > 10 {
    print("Large")
} else if x > 5 {
    print("Medium")
} else {
    print("Small")
}

// Guard statement
guard x > 0 else {
    return .Err(Error.InvalidInput)
}

// While loop
while count > 0 {
    count = count - 1
}

// While-let (Pattern matching loop)
while let .Some(item) = iterator.next() {
    process(item)
}

// Loop (Infinite)
loop {
    if condition { break }
}

// Labeled loops
outer: for i in 0..10 {
    for j in 0..10 {
        if i * j > 50 { break outer }
    }
}

// Match expression (Pattern Matching)
let desc = match direction {
    .North => "Up",
    .South => "Down",
    // Or patterns (Future)
    // .East or .West => "Sideways",
    _ => "Other" // Wildcard
}
```

## Pattern Matching

```kestrel
match value {
    // Literal
    0 => "Zero",
    
    // Enum destructuring
    .Circle(r) if r > 10.0 => "Big Circle", // Guard clause
    
    // Variable binding
    .Rectangle(w, h) => "Rect",
    
    // Wildcard
    _ => "Other"
}

// If-let
if let .Some(val) = optionalVal {
    print(val)
}
```

## Protocols (Traits)

```kestrel
// Protocol definition
protocol Drawable {
    func draw()
}

// Protocol implementation via Extension
extension Shape: Drawable {
    func draw() {
        // Implementation
    }
}

// Generic constraint
func render[T: Drawable](item: T) {
    item.draw()
}

// Built-in attributes
@builtin(.Copyable)
protocol Copyable {}
```

## Closures

```kestrel
// Basic syntax
let add = { (a: Int, b: Int) in a + b }

// Implicit parameters (for single argument) - (Future/Partial)
// let double = { it * 2 }

// Trailing closure syntax
numbers.map { it * 2 }

// Multiple trailing closures
button.setActions(
    onPress: { print("pressed") },
    onRelease: { print("released") }
)
```

## Modules and Imports

```kestrel
// Module declaration
module my_app.utils

// Imports
import std.collections.Map
import std.io as IO // Renaming
```

## Access Control

```kestrel
public struct A { ... }     // Visible everywhere
internal struct B { ... }   // Visible in this module (default)
private struct C { ... }    // Visible in this file/scope
```

## Expressions and Operators

```kestrel
// Ranges
// let range = 1..10   // (Future)
// let inclusive = 1..=10 // (Future)

// Casting - (Future)
// let y = x as Float64

// Nil Coalescing
// let val = optional ?? defaultVal

// Chaining - (Future)
// let x = foo?.bar?.baz
```

## Error Handling

```kestrel
// Function returning Result
func openFile(path: String) -> Result[File, Error] {
    // ...
    .Ok(file)
}

// Try operator - (Future)
// func readConfig() -> Result[Config, Error] {
//     let file = try openFile("config.txt")
//     // ...
//     .Ok(config)
// }
```

## Semicolon Rules

Kestrel uses semicolons to separate statements, but they are often optional in block-like structures.

-   **Required**: After variable declarations (`let x = 1;`) and expression statements (`doSomething();`).
-   **Optional**: After control flow structures (`if`, `while`, `loop`) and for the final expression in a block (implicit return).

```kestrel
func example() -> Int {
    let x = 1;      // Semicolon required
    if x > 0 {      // No semicolon needed
        print("hi");
    }
    x + 1           // No semicolon (implicit return)
}
```
