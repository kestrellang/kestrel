# Access Modes

A parameter's **access mode** declares what the function can do with it. There are three: read-only (the default), `mutating`, and `consuming`. Picking the right one tells the compiler — and the reader — what the function intends.

## Read-only (default)

If you don't write an access mode, the parameter is borrowed read-only. The function can use it but not modify it, and the caller's binding is untouched after the call.

```swift
func area(rect: Rectangle) -> Int {
    rect.width * rect.height
}

let r = Rectangle(width: 3, height: 4)
let a = area(rect: r)   // r is unchanged, still usable
```

This is what you want most of the time. Don't reach for `mutating` or `consuming` unless you have a reason.

## Mutating

`mutating` lets the function write to the parameter. The change is visible to the caller after the function returns. The caller has to pass a `var` binding — passing a `let` is a compile error.

```swift
func reset(mutating counter: Counter) {
    counter.value = 0
}

var c = Counter(value: 99)
reset(counter: c)
// c.value is now 0

let frozen = Counter(value: 99)
// reset(counter: frozen)   // compile error: can't mutate a `let`
```

`mutating` goes before the label, in the parameter list. It pairs with `mutating func` on methods (see [Structs → Methods](../structs/methods.md)) — both are saying the same thing: this code intends to write through the binding.

## Consuming

`consuming` takes the value away from the caller. After a `consuming` call, the caller's binding is no longer usable — the function "owns" the value now.

```swift
func archive(consuming letter: Letter) {
    storage.put(letter)
    // letter is moved into storage; no one else can touch it
}

let l = Letter(...)
archive(letter: l)
// using l here would be a compile error
```

Use `consuming` when the function genuinely takes responsibility for the value: storing it, sending it across a thread boundary, freeing it. For everything else, prefer the default.

## Picking the right one

| Mode | Caller can use after? | Function can write? |
|---|---|---|
| (default) | yes | no |
| `mutating` | yes | yes |
| `consuming` | no | yes (it owns it now) |

A simple rule: start with the default, switch to `mutating` only when you need to write through the binding, switch to `consuming` only when the value semantically *moves*. The compiler will tell you when you've gotten it wrong.

---

[← Functions](index.md) · [↑ Functions](index.md) · [Methods →](methods.md)
