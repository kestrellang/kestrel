# Optional Chaining

`a?.b` reads `b` from the value inside `a` when there is one, and gives
`.None` when there isn't. The whole chain after the first `?.` is skipped on
`.None`, and its result is always an `Optional`.

```kestrel
let len = name?.bytes.count;        // Int64?
let city = user?.address?.city;     // String?
cache?.items.push(entry);           // mutates the payload in place
config?.retries = 3;                // assigns only when config is .Some
counter?.n += 1;
let y = transform?.(x);             // calls an optional function
```

## Syntax

`?.` is `?` written directly against a following `.`. It can start a member
access (`a?.b`), a method call (`a?.m()`), a tuple index (`a?.0`) or a call of
an optional function (`f?.(x)`). Any number of `?.` can appear in one chain.

A chain is the whole postfix expression it's part of: members, calls,
subscripts and `!` after a `?.` belong to it, including `.member` lines
continued on the next line. Anything else ends it: a binary operator (`??`,
`|>`, `==`, …), a closing parenthesis, or the end of the expression.

```kestrel
a?.b.c()        // one chain: c() runs only when a is .Some
(a?.b).c()      // the chain is a?.b; .c() is called on the Optional
a?.b ?? 0       // ?? ends the chain
a?.b |> f       // |> ends the chain: f receives the Optional
a?.b!           // ! unwraps b inside the chain
(a?.b)!         // ! unwraps the chain's result
```

`(x as? T)?.y` needs the parentheses; `x as? T?.y` is a parse error.

## Result type and flattening

The chain's result is `Optional[U]`, where `U` is the type of the tail:

| Tail type | Result |
|-----------|--------|
| `U` (not optional) | `U?` |
| `U?` | `U?` (not `U??`) |
| `()` (a mutation or assignment) | `()?` |
| a generic `T` | `T?`, even when `T` is itself optional |

```kestrel
struct User { var name: String; var nick: String? }

let a = user?.name;   // String?
let b = user?.nick;   // String?, flattened
```

An annotation works like any other expected type:
`let n: Int64? = user?.age`.

## Short-circuiting

Everything after a `?.` runs only when the bind succeeds. That includes call
arguments, later bases and the right side of an assignment:

```kestrel
log?.write(expensiveMessage());    // expensiveMessage() runs only if log is .Some
config?.timeout = computeTimeout(); // computeTimeout() runs only if config is .Some
```

## Borrowing, mutating and taking

A chain never copies its base. What it does with the payload depends on the
base and on what the chain does:

| Base | The chain… | The payload is |
|------|-----------|----------------|
| a place (`var`, `let`, field, parameter) | only reads | borrowed (`&W`) |
| a `var` place | calls a `mutating` member, or assigns through it | borrowed mutably (`&mutating W`); the write lands in the optional itself |
| a temporary (a call result, a literal) | anything | moved out of the temporary |

The compiler decides this after it has resolved the chain's members, so
`a?.push(x)` mutates because `push` is a `mutating` method.

Mutation needs a mutable base, as it does without `?.`. A chain on a `let`
that calls a `mutating` method or assigns is an error (E203).

A chain on a place can't consume the payload (E158). Move it out first:

```kestrel
var token: Token? = load();
let id = token?.spend();          // error[E158]: can't consume through `?.` on a place
let id2 = token.take()?.spend();  // OK: take() leaves .None behind
```

A borrowed payload's value is copied or cloned out like any match-arm value
(see [References](references.md)). A tail that is neither Copyable nor
Cloneable can't be read out of a borrow (E503); match by hand instead.

## The `OptionalChain` protocol

`?.` works on any type that conforms to `OptionalChain` (`std.core`):

```kestrel
public protocol OptionalChain {
    type Wrapped
    func chainBorrow() -> Optional[&Wrapped]
    mutating func chainMutate() -> Optional[&mutating Wrapped]
    consuming func chainTake() -> Optional[Wrapped]
}
```

`Optional` and `Result` conform. On a `Result`, `?.` chains on the `.Ok`
value and the error is dropped:

```kestrel
let r: Result[Point, ParseError] = parse(text);
let x = r?.x;   // Int64?: .Some on .Ok, .None on .Err
```

Generic code can chain through the protocol:

```kestrel
func firstX[O](o: O) -> Int64? where O: OptionalChain, O.Wrapped: HasX {
    o?.x
}
```

`?.` on a type that doesn't conform is E157; use `.` for a value that isn't
optional.

## Interactions

- `try a?.b()` where `b` throws gives `try` an `Optional[Result[…]]`, and
  `try` unwraps the Optional. That is legal but rarely what's meant.
- `xs.map(_?.name)` and `{ it?.name }` are closures whose body is a chain.
- A chain inside a closure that mutates a captured `var` makes the closure a
  mutating one, as any other mutation would.

## Known limitations

- A chain on a *named* `Optional[&T]` (`let r = o.asRef(); r?.x`) reads the
  wrong value today. The generic `chainBorrow` makes a reference to a
  reference there. Chaining on the call directly (`o.asRef()?.x`) works.
  `references/composition/optional_ref_payload_as_ref.ks` and
  `expressions/optional_chaining/chain_ref_base.ks` pin the bug.
