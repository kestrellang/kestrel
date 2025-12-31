# Closures and Capture

Closures in Kestrel capture their environment. The capture behavior interacts with ownership and the Law of Exclusivity.

## Basic Closure Syntax

```kestrel
let add = { (a: Int, b: Int) -> Int in a + b }
let result = add(1, 2)  // 3
```

## Capture Behavior

Closures automatically capture variables from their enclosing scope. The compiler infers how each variable should be captured based on usage:

```kestrel
var x = 10

// x is read, so captured by borrow
let reader = { print(x) }

// x is modified, so captured by mutable borrow
let writer = { x = x + 1 }
```

### Loop Variable Capture

Loop variables are captured **by value**. Each iteration gets its own copy:

```kestrel
var closures: Array[() -> Int] = []
for i in 0..3 {
    closures.push({ i })
}
// closures[0]() returns 0
// closures[1]() returns 1
// closures[2]() returns 2
```

## Non-Escaping Closures

All closures in Kestrel are currently **non-escaping**. A closure cannot outlive its creation scope:

```kestrel
func withValue[T, R](value: T, f: (T) -> R) -> R {
    f(value)
}

var x = 10
withValue(42) { n in
    x = n  // OK: closure doesn't escape, can mutably borrow x
}
```

Non-escaping closures:
- Can borrow from the environment
- Are stack-allocated (no heap allocation)
- Cannot be stored or returned from functions

```kestrel
// ERROR: Cannot return a closure (escaping not supported)
func makeCounter() -> () -> Int {
    var count = 0
    return { count = count + 1; count }  // Not allowed
}
```

---

## Potential Issues

### 1. Closure Capture and Exclusivity

Multiple closures capturing the same variable:

```kestrel
var x = 10
let a = { x = 1 }
let b = { x = 2 }
a()
b()  // Both exist and mutate x - is this allowed?
```

**Rule**: The Law of Exclusivity applies. You cannot simultaneously have two active mutable accesses. Creating and immediately calling closures sequentially is fine; holding two mutable closures and calling them interleaved may violate exclusivity.

### 2. Nested Closures

What happens with nested closures?

```kestrel
var x = 10
let outer = {
    let inner = { x = 20 }  // inner captures x
    inner()
}
```

**Expected**: `outer` must capture `x` (transitively) for `inner` to access it.

### 3. Recursive Closures

Self-referential closures are tricky:

```kestrel
let factorial = { (n: Int) -> Int in
    if n <= 1 { 1 }
    else { n * factorial(n - 1) }  // ERROR: factorial not yet defined
}
```

**Workarounds**:
- Use `var` and assign later
- Define as a named function instead
