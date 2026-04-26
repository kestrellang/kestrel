# Closures

A closure is a function without a name. You can write one inline, store it in a variable, or pass it to another function.

## Basic syntax

A closure is wrapped in braces. Parameters come before `in`; the body comes after:

```swift
let double = { (x: Int) in x * 2 }
let result = double(5)   // 10
```

When the type is known from context, you can drop the parameter type:

```swift
let nums = [1, 2, 3]
let doubled = nums.map({ (x) in x * 2 })
```

## The `it` shorthand

For a single-parameter closure where the type is inferred, you can drop the parameter list entirely and refer to the argument as `it`:

```swift
nums.map { it * 2 }
nums.filter { it > 0 }
```

This makes pipelines read like prose. Use named parameters when the body is long enough that `it` becomes unclear.

## Trailing closures

When a closure is the last argument, you can write it after the parentheses:

```swift
func retry(times: Int, action: () -> Bool) -> Bool { /* ... */ }

retry(times: 3) {
    network.ping()
}
```

If the closure is the *only* argument, you can drop the parentheses:

```swift
let value = compute { expensiveOperation() }
```

This is the syntax that makes Kestrel's `for`-like APIs (`each`, `map`, `filter`) feel built-in even though they're just functions.

## Capture

Closures capture values from the surrounding scope by value — the binding is copied at the moment the closure is created:

```swift
var counter = 0
let snapshot = { counter }

counter = 99
let value = snapshot()   // 0, not 99
```

If you need a closure to reflect later changes, capture a reference type or use `var` shared state explicitly.

## Closures as values

A closure has a function type written `(Args) -> Return`:

```swift
let op: (Int, Int) -> Int = { (a, b) in a + b }

func apply(f: (Int, Int) -> Int, a: Int, b: Int) -> Int {
    f(a, b)
}

apply(f: op, a: 3, b: 4)   // 7
```

This is what lets functions take behavior as data — the foundation of `map`, `filter`, callback APIs, and the iterator chain.

---

[← Methods](methods.md) · [↑ Functions](index.md) · [Operator Overloading →](operator-overloading.md)
