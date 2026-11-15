# Indirection

A type that conforms to `Indirection` acts like the value it points to. If you
access a member the wrapper doesn't have, the access goes through to the
wrapper's target. If you pass the wrapper where a borrowed target is expected,
the target is passed. This is how `Array` gets every `Slice` method, how
`String` gets every `StringSlice` method, and how `RcBox`, `CowBox` and
`OwnedBox` feel like the value they hold. The compiler calls this step a
**peel**.

## On this page

- [The protocols](#the-protocols)
- [A worked example](#a-worked-example)
- [The rules](#the-rules)
- [What doesn't peel](#what-doesnt-peel)
- [Converting a view to an owned value](#converting-a-view-to-an-owned-value)
- [Where the std uses it](#where-the-std-uses-it)
- [Errors](#errors)

## The protocols

```kestrel
public protocol Indirection {
    type Target: not Sized
    var value: Target { ref }                // reads peel through this
}

public protocol MutableIndirection: Indirection {
    var value: Target { ref mutating ref }   // writes peel through `mutating ref`
}
```

- **`Indirection`** lends its target for reading. Reads, non-`mutating` method
  calls and borrowed arguments peel.
- **`MutableIndirection`** also lends its target for writing. Field writes,
  compound assignments (`w.n += 1`), `mutating` method calls and `mutating`
  arguments peel too. The wrapper itself must be a `var`.
- **`ConsumingIndirection`** is for a single-owner box that can give up its
  payload (`OwnedBox`). Existentials use it for `consuming` requirements and
  `consuming x as? T`. See [existentials.md](existentials.md).

`Target` may be unsized, so a wrapper can lend a `Slice[T]` or a
`StringSlice`.

## A worked example

```kestrel
module Accounts

struct Account {
    var owner: String
    var balance: Int64

    func summary() -> String { "\(self.owner): \(self.balance)" }
    mutating func deposit(amount: Int64) { self.balance += amount; }
}

struct Handle: MutableIndirection {
    var account: Account
    var label: String

    type Target = Account
    var value: Account {
        ref { self.account }
        mutating ref { self.account }
    }

    // The wrapper's own member wins a name clash.
    func summary() -> String { "handle \(self.label)" }
}

func audit(a: &Account) -> Int64 { a.balance }
func topUp(mutating a: Account) { a.deposit(1); }

@main
func main() -> Int64 {
    var h = Handle(account: Account(owner: "ana", balance: 10), label: "h1");
    let b = h.balance;              // read peels to Account.balance
    h.balance += 5;                 // write peels through `mutating ref`
    h.deposit(3);                   // a mutating method peels too
    let mine = h.summary();         // Handle.summary: the wrapper wins
    let theirs = h.value.summary(); // Account.summary: reach through explicitly
    let seen = audit(h);            // a borrowed argument peels
    topUp(h);                       // so does a mutating one
    println("\(b) \(mine) \(theirs) \(seen) \(h.balance)");
    if h.balance != 19 { return 1; }
    0
}
```

This prints `10 handle h1 ana: 18 18 19`.

## The rules

1. **The wrapper wins.** A member access first looks at the wrapper's own
   members. The peel only happens when that lookup misses, so a wrapper member
   with the same name as a target member always takes priority, and there is no
   ambiguity error. To reach the target's member anyway, write `w.value.m`.
2. **The use picks the accessor.** A read goes through `value`'s `ref`. A
   write, a compound assignment or a `mutating` call goes through its
   `mutating ref`, which needs `MutableIndirection` and a `var` wrapper. That is
   how `CowBox` copies on write: its `mutating ref` forks shared storage first,
   while reads don't.
3. **Borrowed arguments peel.** If a parameter takes `x: T`, `x: &T` or
   `mutating x: T` and you pass a wrapper whose `Target` is `T`, the wrapper's
   target is passed in place. This works for functions, methods, inits and
   defaulted parameters. An overload that matches exactly wins over one that
   only fits through the peel.
4. **Peels chain, one level at a time.** If the target is itself an
   `Indirection`, a member missing on it peels again, so
   `RcBox[RcBox[Account]].balance` reaches `Account.balance`. The chain stops
   at the first target that isn't an `Indirection`.
5. **Copies and drops are unaffected.** Copying a wrapper copies the wrapper
   (an `RcBox` copy bumps the refcount), and the wrapper drops as itself. A
   value read through the peel is a copy of that member, following the
   member's own copy rules.

## What doesn't peel

- **Consuming parameters, bindings and returns.** `take(h)` where `take` takes
  `consuming a: Account`, `let t: Account = h` and `return h` from a function
  returning `Account` are all type mismatches (E109). Write `h.value`, or clone
  it, to get the target.
- **Generic parameters.** A `T` parameter binds the wrapper type itself.
- **Operators and protocol conformances.** `==`, `<`, `+`, hashing,
  interpolation and every other protocol requirement look for a conformance on
  the wrapper, not the target. Forward one explicitly, for example
  `extend Handle: Equatable { … }`, or the `extend &T: P where T: P` pattern the
  std uses for references.

## Converting a view to an owned value

Views such as `&StringSlice` and `&Slice[T]` never turn into owned values on
their own. They convert only where the target names an owned type that
conforms to `DecaysFrom[View]`: an annotation, a parameter, a field, a return
type, an assignment target, or an annotated collection element.

```kestrel
let text = "  hi  ";
let word = text.trimmed();          // &StringSlice, borrows text
let owned: String = text.trimmed(); // String, built by DecaysFrom
```

A `mutating` parameter never converts, because the writes would land in a
temporary. A borrowed `String` parameter that is given a view still converts,
but the compiler warns (E512) that it copies; take `&StringSlice` instead.

## Where the std uses it

| Wrapper | `Target` | Effect |
|---|---|---|
| `Array[T]` | `Slice[T]` | `&Slice[T]` params take arrays, and every `Slice` method works on an array |
| `UnsafeSlice[T]` | `Slice[T]` | Same, for raw memory |
| `String` | `StringSlice` | `&StringSlice` params take strings, and every `StringSlice` method works on a string |
| `RcBox[T]` | `T` | Shared box; reads and writes reach the payload |
| `CowBox[T]` | `T` | Writes fork shared storage first (needs `T: Cloneable` for writes) |
| `OwnedBox[T]` | `T` | Single-owner box; also `ConsumingIndirection` |

For which parameter type to take in your own functions, see
[references.md](references.md).

## Errors

| Code | When |
|---|---|
| E102 | The member exists on neither the wrapper nor its target |
| E208 | A write or `mutating` call through a wrapper that is only `Indirection` |
| E109 | A wrapper passed to a `consuming` parameter, bound, or returned as its target |
| E512 (warning) | A view passed to a borrowed owned-type parameter, which copies it |

The design record, with the compiler-side architecture, is in
[`docs/plans/indirection/`](../plans/indirection/README.md).
