# Errors and their usual fixes

Each error below was reproduced on the current compiler. The message is the
first line `kestrel build` prints. Codes are stable, but message wording may
drift, so match on the code.

## Calls, labels, names

| Code | Message starts with | Usual cause | Fix |
|---|---|---|---|
| E113 | `'x' is the parameter's name, not an argument label` | Passing a label to a single-name param: `add(x: 1, y: 2)` | Drop it: `add(1, 2)`. Same in std: `File.readText(path)`, `d.remove("k")` |
| E113 | `wrong argument label` / `no matching overload` | Wrong label, or a trailing closure on a labeled closure param | Spell the label: `filter(where: { … })`, `fold(from: 0, by: { … })` |
| E102 | `no member 'count' on type 'String…'` | Calling a member that doesn't exist, often a remembered name | Grep the std for the real one (here: `s.bytes.count`) |
| E104 | `implicit member '.B' not found` | `.Case` shorthand naming a case the expected type lacks | Check the case name and that the expected type is the enum you think |
| E476 | `cannot find type 'Float16' in this scope` | Type name doesn't exist or isn't imported (deps need an `import`) | Fix the name or add `import pkg.module.(Type)` |
| E414 | `this file has no \`module\` declaration` | Missing first line. Often followed by hundreds of std errors. | Add `module Name` at the top and rebuild before reading the rest |
| E144 | `protocol 'P' cannot be used as a type` | `func g(p: P)` | `p: any P` (boxed) or `p: some P` (opaque) |

## Types and inference

| Code | Message | Usual cause | Fix |
|---|---|---|---|
| E109 | `type mismatch` | A value of the wrong type where another was expected, such as `f("a")` for `f(x: Int64)` | Convert explicitly (`Int64(parsing:)`, `.toOwned()`, …) |
| E107 | `type mismatch: does not conform to protocol…` | A literal or value can't become the expected type: `let x: Int64 = 1.5` | Use the right literal or convert |
| E105 | `could not infer type` | Nothing pins a type, e.g. `let xs = [];` | Annotate: `let xs: Array[Int64] = [];` |
| E305 | `non-exhaustive match: missing .B` | A `match` without every case | Add the arms, or `_ => …` |

## Mutability and ownership

| Code | Message | Usual cause | Fix |
|---|---|---|---|
| E200 | `cannot assign to immutable variable 'a'` | Assigning to a `let` | Declare it `var` |
| E203 | `cannot pass immutable binding 'a' to 'mutating' parameter` | `let` passed to a `mutating` param or receiver | Make it `var` |
| E210 | ``cannot take a `&mutating` borrow of immutable variable`` | `&mutating x` on a `let` | Make it `var` |
| E500 | `use of moved value 'h'` | Using a `not Copyable` value after passing it `consuming` or binding it elsewhere | Use it before the move, pass it borrowed, or `clone()` it if it's `Cloneable` |
| E503 | `cannot move non-copyable value … out of a borrow` | Returning or binding a move-only field from a borrowed param | Take the param `consuming`, return a reference, or clone |
| E507 | `cannot consume 'h…' while a closure capturing it is live` | Moving a value that a live closure views | Finish with the closure first, or capture it with an `escaping`/`consuming` closure that owns it |

## References and closures

| Code | Message | Usual cause | Fix |
|---|---|---|---|
| E488 | ``a borrow expression is only allowed as a `let` initializer`` | Writing `f(&x)` | Drop the `&`; arguments borrow by signature |
| E497 | `a reference cannot stay live across a control-flow merge` | A slice or ref result from `if`/`match` arms, or a ref crossing a loop | Bind it before the branch, or make each arm return an owned value (`String`, `Array`) |
| E493 | `ambiguous borrow source for the returned reference` | A ref-returning function where the compiler can't tell which param the result borrows | Return from a single param on a straight line, or return an owned value |
| E512 (warning) | `passing a 'StringSlice' to a borrowed 'String' parameter copies it` | A `String` param fed a slice | Change the param to `&StringSlice` |
| E494 | `cannot return this closure: it captures …` | Returning or storing a normal closure that captures | Write the return type `escaping () -> T` |
| E624 | `closure kind mismatch: expected an 'escaping' closure, found a normal closure` | Passing an already-bound normal closure where `escaping` is expected | Annotate where it's created: `let g: escaping () -> Int64 = { … }` |
| E603 | `cannot assign to captured variable 'x'` | Writing a capture inside a normal closure | Expect a `mutating` closure type instead |
| E153 | `an 'is' pattern binds a borrow…, so it cannot be 'var'` | `var s is S` | Use `s is S` and copy out if you need to mutate |

## Reading the output

- Fix the **first** error and rebuild. Later ones are often consequences,
  especially after E414, E476 and E105.
- A diagnostic located inside the std usually means your call passed the
  wrong type. Look at the call site it names in your file.
