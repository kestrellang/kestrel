# Control Flow

Branching and looping. Most control-flow constructs are expressions in Kestrel — they produce a value — which means you can use them on the right side of `let`.

## If / Else

```swift
if user.isAdmin {
    grantAccess()
} else if user.isMember {
    grantLimitedAccess()
} else {
    denyAccess()
}
```

Because `if` is an expression, you can assign its result:

```swift
let label = if score > 90 {
    "excellent"
} else if score > 70 {
    "good"
} else {
    "needs work"
}
```

Each branch must produce a value of the same type. Unless one of them diverges — calls a `!`-returning function, breaks, or returns — in which case it's allowed to be a different shape, since execution will never get past it. See [Functions → Return Types](functions/index.md#return-types).

## Loops

`while` runs as long as a condition holds:

```swift
var i = 0
while i < 10 {
    println("\(i)")
    i = i + 1
}
```

`loop` is unconditional — exit it with `break`:

```swift
loop {
    let line = read()
    if line.isEmpty() { break }
    process(line)
}
```

`for` walks an iterable:

```swift
for item in cart.items {
    total = total + item.price
}
```

`continue` skips to the next iteration. `break` exits the loop. Both can target an outer loop with a label:

```swift
search: for row in rows {
    for cell in row.cells {
        if cell.matches(target) {
            break search
        }
    }
}
```

## Guard

`guard` is the early-exit pattern. It checks a condition and, if it fails, runs an `else` block that must leave the surrounding scope (return, throw, break, or call a `!`-returning function).

```swift
func process(input: String) -> Result[Output, Error] {
    guard input.isNotEmpty() else {
        return .Err(.EmptyInput)
    }

    guard let .Some(parsed) = parse(input) else {
        return .Err(.ParseFailed)
    }

    // parsed is in scope here, unwrapped
    .Ok(transform(parsed))
}
```

Compared to `if`, `guard` keeps the happy path at the outer indentation level. Use it when "the rest of this function depends on this being true."

## Match

`match` checks a value against patterns. The first matching arm runs.

```swift
match status {
    .Active => println("running"),
    .Paused(reason) => println("paused: \(reason)"),
    .Stopped => println("done"),
}
```

Like `if`, `match` is an expression — every arm produces a value:

```swift
let label: String = match status {
    .Active => "live",
    .Paused(_) => "halted",
    .Stopped => "off"
}
```

The compiler checks that every case is handled. Add a new variant to the enum and every existing `match` lights up red until you cover it. Use `_` as a catch-all when you genuinely want to default; don't use it to silence the exhaustiveness checker. For the deeper pattern-matching story — destructuring, guards, bindings — see [Enums → Pattern Matching](enums/pattern-matching.md).

---

[← Operator Overloading](functions/operator-overloading.md) · [↑ The Kestrel Language](index.md) · [Collections →](collections/index.md)
