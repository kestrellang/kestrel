# Kestrel Compiler Error Codes

> Generated from the compiler source (diagnostic descriptors in `lib/`) on
> 2026-07-01, updated 2026-07-17. Every code below corresponds to a descriptor
> or emit site in the compiler; message templates use `{name}`-style
> placeholders for interpolated values. Unless marked *(warning)*, a code is an
> error. Each code maps to exactly one diagnostic (enforced by a unit test in
> `kestrel-analyze`).

Most diagnostics print as `message [E-code]` with source labels. Inference
errors are all surfaced under the umbrella code **E100**; the detailed message
comes from the type checker.

## Contents

- [E001–E009 — Control flow & definite initialization](#e001e009--control-flow--definite-initialization)
- [E100–E121 — Type checking, parameters & literals](#e100e121--type-checking-parameters--literals)
- [E200–E211 — Mutability, access modes & assignment](#e200e211--mutability-access-modes--assignment)
- [E300–E316 — Patterns & exhaustiveness](#e300e316--patterns--exhaustiveness)
- [E400, E411–E479 — Declarations, generics & protocol conformance](#e400-e411e479--declarations-generics--protocol-conformance)
- [E480–E499 — References & escape checking](#e480e499--references--escape-checking)
- [E500–E507 — Moves & ownership](#e500e507--moves--ownership)
- [E600–E614, E623 — Closures, externs & declaration shape](#e600e614-e623--closures-externs--declaration-shape)
- [E615–E618 — Entry point](#e615e618--entry-point)
- [E619–E622 — Place accessors](#e619e622--place-accessors)
- [E624–E625 — Closure kinds](#e624e625--closure-kinds)
- [E700–E707 — String literals & escapes](#e700e707--string-literals--escapes)
- [E800–E809 — Syntax](#e800e809--syntax)

---

## E001–E009 — Control flow & definite initialization

| Code | Message | Explanation |
|---|---|---|
| E001 | function '{name}' does not return a value on all code paths | A function with a return type has a path that falls off the end without returning. |
| E002 *(warning)* | unreachable code | Code after a `return`, `break`, `continue`, `throw`, or other diverging statement can never run. |
| E003 | guard else block must diverge (return, break, continue, or throw) | The `else` of a `guard` must exit the scope; execution cannot fall through it. |
| E004 | access to uninitialized variable '{name}' | A variable is read before any assignment gives it a value. |
| E005 | initializer does not initialize all fields: {field_list} | An `init` returned without assigning every stored field. |
| E006 | cannot assign to 'let' field '{name}' more than once | Inside `init`, a `let` field may only be assigned once. |
| E007 | cannot read field '{name}' before it is initialized | An `init` body reads a field that has not been assigned yet. |
| E008 | cannot use 'self' before all fields are initialized | Whole-`self` uses (method calls, passing `self`) require every field to be set first. |
| E009 | cannot return before all fields are initialized | An `init` has an early `return` while some fields are still unassigned. |

## E100–E121 — Type checking, parameters & literals

| Code | Message | Explanation |
|---|---|---|
| E100 | type mismatch: {detail} | Umbrella code for all type-inference errors (mismatched types, unknown members, ambiguous overloads, unsatisfied bounds, …). The detail text comes from the solver. |
| E101 | {kind} condition must be Bool | An `if`/`while`/`guard` condition has a non-`Bool` type. |
| E110 | duplicate binding '{name}' in parameter pattern | The same name is bound twice in one parameter's destructuring pattern. |
| E111 | tuple pattern has {n} elements but type has {m} | A tuple-destructuring parameter doesn't match the tuple type's arity. |
| E112 | tuple pattern used with non-tuple type | A parameter uses tuple destructuring but its type is not a tuple. |
| E121 | integer literal out of range for \`{type}\` | The literal cannot be represented by the resolved integer type; it would silently wrap. |

### Example — E100 (type mismatch)

```kestrel
func pick(cond: Bool) -> Int64 {
    if cond { 42 } else { "hello" }  // error[E100]: both branches must have the same type
}
```

### Example — E121 (integer literal out of range)

```kestrel
let tiny: Int8 = 200  // error[E121]: integer literal out of range for `Int8`
                      // Int8 holds -128...127
```

## E200–E211 — Mutability, access modes & assignment

| Code | Message | Explanation |
|---|---|---|
| E200 | cannot assign to immutable variable '{name}' | Assignment to a `let` binding; declare it `var` to mutate. |
| E201 | cannot assign to immutable field '{name}' | Assignment to a `let` field, or to a field without a setter. |
| E202 | cannot assign to this expression | The left-hand side of the assignment is not an assignable place (e.g. a call result or literal). |
| E203 | cannot pass immutable binding '{name}' to 'mutating' parameter · cannot call `mutating` closure '{name}': it is bound with 'let' | A `let` binding can't be handed to a parameter that mutates it. Calling a `mutating`-kind closure is an exclusive use of whatever holds it, so it must live in a `var` (or a `mutating` parameter). |
| E204 | cannot pass immutable field '{name}' to 'mutating' parameter | An immutable field can't be passed where mutation is required. |
| E205 | cannot pass temporary value to 'mutating' parameter | A temporary (rvalue) has no place to write the mutation back to. |
| E206 | *(declared; not currently emitted)* | Reserved: passing a `let` binding to a `consuming` parameter. |
| E207 | cannot call a 'mutating' method through a shared reference | A `&T` reference is read-only; mutating calls need `&mutating` or an owned `var`. |
| E208 | cannot assign through a shared reference | Writing through `&T` is not allowed; take `&mutating` instead. |
| E209 | a ref binding must be a simple \`let\` | A borrow initializer (`let r = &x`) can't be combined with `var` or a destructuring pattern. |
| E210 | cannot take a \`&mutating\` borrow of immutable variable '{name}' | Mutable borrows need a mutable place (`var`, mutable field, or `Pointer.mutatingValue`). |
| E211 | \`&\` pattern bindings are not supported in this position | `&` patterns are only allowed in match-arm patterns, not in `let`/`for`/conditions/params. |
| E212 | *(retired)* | Was "closure cannot capture non-Static binding '{name}'". Retired with closure kinds (docs/design/closures.md): a view-kind closure's environment is frame-bound, so it may capture ref bindings and non-`Static` values freely. The owning tier keeps the rejection under E624. |

## E300–E316 — Patterns & exhaustiveness

| Code | Message | Explanation |
|---|---|---|
| E300 | refutable pattern in let binding | A `let` pattern must always match; use `if let` / `guard let` for patterns that can fail. |
| E301 | refutable pattern in for-loop binding | A `for` loop pattern must match every element the iterator produces. |
| E302 *(warning)* | this pattern always matches | An `if let` whose pattern is irrefutable — the condition is pointless. |
| E303 *(warning)* | this pattern always matches | A `match` arm before the end that always matches makes later arms dead. |
| E304 | empty match on inhabited type | A `match` with no arms is only legal on uninhabited types. |
| E305 | non-exhaustive match: missing {cases} | The `match` does not cover every possible value; the message lists missing cases. |
| E306 *(warning)* | unreachable pattern | This pattern can never match because earlier arms cover it. |
| E307 *(warning)* | overlapping range patterns | Two range patterns overlap; the later one is partly shadowed. |
| E308 *(warning)* | this pattern always matches | An irrefutable `while let` loops forever (or should be `while true`). |
| E309 *(warning)* | this pattern always matches | An irrefutable `guard let` never takes the `else` branch. |
| E310 | duplicate binding '{name}' in pattern | The same name is bound twice within a single pattern. |
| E311 | float literal in pattern | Float equality is unreliable; match on an integer or a range instead. |
| E312 | unknown enum case '{name}' on type '{ty}' | The pattern names a case the enum doesn't have. |
| E313 | variant '{name}' takes {expected} argument(s), got {got} | Payload pattern arity doesn't match the enum case. |
| E314 | tuple pattern has {pat} elements but type has {ty} | Tuple pattern arity doesn't match the matched tuple type. |
| E315 | inconsistent bindings across or-pattern alternatives | Every `|` alternative must bind the same names with the same types. |
| E316 *(warning)* | match on \`String\` with {n} literal arms does byte-equality per arm | Large string matches are O(arms × len); consider a different dispatch strategy. |

### Example — E305 (non-exhaustive match)

```kestrel
func unwrap(opt: Int64?) -> Int64 {
    match opt {          // error[E305]: non-exhaustive match: missing null case
        some x => x
    }
}
```

## E400, E411–E479 — Declarations, generics & protocol conformance

| Code | Message | Explanation |
|---|---|---|
| E400 | duplicate @builtin(.{feature}): already declared by '{name}' | Two declarations carry the same `@builtin` annotation. A lang item is identified by its annotation alone, so only the first (in declaration order) is used. |
| E411 | duplicate method '{name}': defined on both the type and an extension | An extension redefines a method the type already declares. |
| E412 | duplicate method '{name}' in extensions of '{type}' | Two extensions of the same type instantiation define the same method. |
| E413 | computed properties must use 'var' | A computed property (with `get`/`set`) can't be declared `let`. |
| E415 | enums cannot have stored fields | Enums carry data in case payloads, not stored fields. |
| E416 | static stored properties not supported in generic types | A generic type would need one global per instantiation; use a computed static instead. |
| E417 | protocol method '{method}' in '{protocol}' cannot have a body | Protocol requirements are signatures only; put default bodies in a protocol extension. |
| E418 | '{name}' cannot be static in this context | `static` is not allowed at module level. |
| E419 | @builtin(.{feature}) must be a marker protocol (no required methods or types) | Builtin language-feature protocols cannot declare requirements. |
| E420 | fields of '{type_name}' do not conform to '{protocol}' | A field-checked conformance (e.g. `FFISafe`) requires every stored field to conform too. |
| E421 | '{type_name}' conforms to '{child_protocol}' but not its parent '{parent_protocol}' | Conforming to a child protocol requires conforming to the protocols it inherits from. |
| E422 | enum '{enum_name}' cannot conform to protocol '{protocol_name}' | This protocol is restricted to non-enum types. |
| E423 | cannot conform to \`{protocol_name}\` and opt out of \`Copyable\` | A conformance that requires `Copyable` conflicts with `: not Copyable`. |
| E424 | '{name}' is not a language feature protocol | `not P` (negative conformance) is only allowed for language-feature protocols like `Copyable`. |
| E425 | '{type}' conforms to Copyable but contains non-Copyable field '{field}' | A `Copyable` type must be copyable field-by-field. |
| E426 | duplicate {kind} signature: {signature} | Two functions/inits/subscripts have identical signatures — overloads must differ. |
| E427 | duplicate enum case '{case_name}' | The same case name is declared twice in one enum. |
| E428 | duplicate label '{label}' in case '{case_name}' | An enum case payload uses the same label twice. |
| E429 | enum '{name}' is recursive without 'indirect' | A case payload contains the enum itself; recursion needs `indirect` (heap boxing). |
| E430 | return type of '{name}' is less visible than the function | A `public` function can't expose a non-public type in its return. |
| E431 | parameter type in '{name}' is less visible than the function | A `public` function can't take a non-public parameter type. |
| E432 | aliased type in '{name}' is less visible than the type alias | A `public` alias can't point at a non-public type. |
| E433 | field '{name}' has type less visible than the field | A `public` field can't have a non-public type. |
| E434 | duplicate type parameter name '{name}' | The same generic parameter name appears twice in one parameter list. |
| E435 | type parameter '{without}' without default follows '{with_default}' which has a default | Defaulted type parameters must come last. |
| E436 | bound '{type_name}' is a {type_kind}, not a protocol | Generic bounds (`T: X`) must name protocols. |
| E437 | undeclared type parameter '{name}' in where clause | The `where` clause constrains a name that isn't a type parameter in scope. |
| E438 | too few/too many type arguments for '{name}': expected {n}, got {m} | Wrong number of generic arguments (also: the type doesn't accept arguments at all). |
| E439 | type parameter '{name}' shadows outer type parameter | A nested declaration reuses an enclosing generic parameter's name. |
| E440 | no associated type '{name}' on '{type}' | A `where` clause projects an associated type the bound protocol doesn't declare — as a bound subject (`T.X: P`) or as an equality's left side (`T.X = Y`), where no declared bound of `T` and no bound in the same clause declares `X`. |
| E441 | type alias '{name}' cannot have bounds outside a protocol | Bounded `type` aliases (associated types) belong in protocol bodies only. |
| E442 | type alias '{name}' requires a type definition | Outside a protocol, `type X` needs `= SomeType`. |
| E443 | '{type_name}' does not conform to '{protocol_name}' | A qualified associated-type binding (`T.[P].X`) names a protocol `T` doesn't conform to. |
| E444 | protocol '{protocol}' has no associated type '{type_name}' | A qualified binding names a member the protocol doesn't declare. |
| E445 | associated type '{name}' is ambiguous between protocols: {list} | Multiple conformed protocols declare the same associated type; qualify it. |
| E446 | type '{bound_type}' does not satisfy constraint '{protocol}' on associated type '{name}' | The concrete binding of an associated type violates the protocol's bound on it. |
| E447 | circular type alias: '{A}' -> ... -> '{A}' | Type aliases form a resolution cycle. |
| E448 *(warning)* | *(reserved; not currently emitted)* | Type alias contains an inferred type. |
| E449 | struct '{name}' cannot contain itself | A struct storing itself by value would have infinite size. |
| E450 | circular struct containment: '{A}' -> ... -> '{A}' | Structs contain each other by value in a cycle. |
| E451 | circular generic constraint: '{A}' -> ... -> '{A}' | `where` clauses form a constraint cycle. |
| E452 | cannot extend '{name}' | Only structs, enums, and protocols can be extended (or the target type doesn't exist). |
| E453 | wrong number of type parameters for '{name}': expected {n}, got {m} | An extension's generic parameter count doesn't match the target type. |
| E454 | type '{type}' does not implement method '{method}' from protocol '{proto}' | A declared conformance is missing a required method. |
| E455 | type '{type}' does not provide associated type '{name}' from protocol '{proto}' | A conformance is missing a required associated-type binding. |
| E456 | property '{name}' has wrong type for protocol '{proto}' | A witness property's type doesn't match the protocol requirement. |
| E457 | type '{bound}' does not satisfy bound '{proto}' on associated type '{name}' | The associated type chosen for a conformance breaks the protocol's `where` bound. |
| E458 | method '{name}' has wrong return type for protocol '{proto}' | A witness method's return type doesn't match the requirement. |
| E459 | circular protocol inheritance: '{A}' -> ... -> '{A}' | Protocols inherit from each other in a cycle. |
| E460 | property '{name}' requires a setter to satisfy protocol '{proto}' | The protocol requires `get set` but the witness property is read-only. |
| E461 *(warning)* | unknown attribute '{name}' | An `@attribute` isn't recognized by the compiler. |
| E462 | conflicting associated type '{name}' inherited by protocol '{proto}' | Two inherited protocols declare incompatible versions of the same associated type. |
| E463 | method '{name}' is ambiguous: satisfies requirements of multiple protocols ({list}) | One method would witness several unrelated protocols; implement each conformance in its own `extend Type: Protocol` block. |
| E464 | init has wrong effect for protocol '{proto}' | A witness `init`'s failability/throwing effect doesn't match the protocol requirement. |
| E465 | indirect enums are not yet supported | `indirect` is recognized but not implemented in this version. |
| E466 | 'some' (opaque type) is not allowed in a field type | Opaque `some P` types can only appear in return position. |
| E467 | method '{name}' shadows the stored field '{name}' of '{type}' | A zero-arg extension method with a stored field's name would recurse instead of reading the field. |
| E473 | struct \`{name}\` already has a deinit | A type may declare at most one `deinit`. |
| E474 | duplicate definition of {kind} '{name}' | Two declarations of the same kind share a name in the same scope. |
| E475 | '{name}' is already defined as a {original_kind} | A name is reused by a declaration of a different kind (e.g. struct vs func). |
| E476 | cannot find type '{name}' in this scope | A type annotation names a type that doesn't resolve. |
| E477 | method '{name}' has wrong receiver kind for protocol '{proto}' | The witness's receiver (`mutating`/`consuming`/plain) doesn't match the requirement. |
| E478 | 'static' is redundant here | Global (module-level) properties are already static. |
| E479 | associated type '{name}' in where clause is ambiguous: '{type}' is bound by {protocols}, which each declare '{name}' | An equality clause `T.X = Y` where two or more protocols bound on `T` in the same clause each declare `X`. A where-clause path has no protocol-qualified form, so the clause is ignored. |

### Example — E412 (duplicate extension method)

```kestrel
struct Box { var value: Int64 }

extend Box { func show() -> String { "a" } }
extend Box { func show() -> String { "b" } }  // error[E412]: duplicate method 'show'
                                              // in extensions of 'Box'
```

### Example — E429 (recursive enum)

```kestrel
enum Tree {
    case leaf(Int64)
    case node(Tree, Tree)  // error[E429]: enum 'Tree' is recursive without 'indirect'
}
```

### Example — E454 (missing protocol method)

```kestrel
protocol Describable { func describe() -> String }

struct Point { var x: Int64; var y: Int64 }

extend Point: Describable { }  // error[E454]: type 'Point' does not implement
                               // method 'describe' from protocol 'Describable'
```

## E480–E499 — References & escape checking

References (`&T` / `&mutating T`) are second-class: they can't be stored, and
a returned reference must outlive the call. The escape checker (E494–E498)
runs on MIR and tracks each reference's *root provenance* — including
references and closures laundered through structs, enums, and tuples.

**E480–E487 and E489 are positional rejections** — "a reference type is not
allowed *here*". They are emitted from HIR lowering
(`kestrel-hir-lower/src/ty.rs`, `RefPosition::code_and_message`), not by an
analyzer, so they carry a codespan code rather than a registry descriptor.
Which of them can fire depends on the entry point's `RefPolicy`: aggregate
positions are legal since stage 2b except from STRICT entries (type-alias RHS,
protocol/extension-target arguments, where-clause types).

| Code | Message | Explanation |
|---|---|---|
| E480 | parameters are not reference-typed; spell the convention instead | **Permanent.** `x: T` borrows and `mutating x: T` mutably borrows — conventions are the only spelling. Covers function-type and closure parameters too. |
| E481 | reference return types are not supported yet | Legal since stage 1; the code is retained for the positions still rejected. |
| E482 | references cannot be stored in bindings | A `var`/`let` annotation may not be `&T`. An aggregate that *wraps* a reference is legal. |
| E483 | references cannot be stored in fields | Struct/enum fields, including enum case payloads (payloads classify as Field, not Param). Legal since 2b except from STRICT entries. |
| E484 | references cannot be stored in tuples | Tuple elements. Legal since 2b except from STRICT entries. |
| E485 | references cannot be used as type arguments | Generic arguments (`Array[&T]`). Legal since 2b except from STRICT entries. See also E492, the inference-time form. |
| E486 | reference returns are not supported in function types yet | `() -> &T` as a *type*, not a declaration. |
| E487 | a reference cannot reference a reference | `&&T` / `&mutating &T`. Reported once for the whole cluster; fixing the nesting then surfaces the positional error, if any. |
| E489 | reference types cannot be used here | The catch-all position — alias RHS, where-clause types, protocol bounds. |
| E488 | a borrow expression is only allowed as a \`let\` initializer | `&x` is not a free-standing expression; parameters already borrow by signature. |
| E490 | a throwing function cannot return a reference | `throws` wraps the return in `Result`, and a reference can't live in an enum payload. |
| E491 | a reference-returning function cannot be used as a value | `-> &T` is a return convention, not part of a function type; call it instead of storing it. |
| E492 | a reference cannot be a generic type argument | That would store the reference; bind the value first to store an owned copy. |
| E493 | ambiguous borrow source for the returned reference | The compiler can't tell which parameter the returned `&T` borrows from. |
| E494 | cannot return this reference/closure: it borrows local '{name}', which does not outlive the call | A reference (or a closure/value carrying one) rooted in a local would dangle after return. |
| E495 | returning \`&mutating\` requires a mutable root | A `&mutating` return needs a `mutating` receiver/parameter or `Pointer.mutatingValue` as its root; statics and `.value` don't qualify. |
| E496 | cannot return a reference rooted at consuming parameter '{name}': it is destroyed when the call returns | Consuming parameters die with the call, so references into them can't escape. |
| E497 | a reference cannot stay live across a control-flow merge in this version | A reference may not cross an `if`/`match`/loop boundary (current-version limitation). |
| E498 | cannot consume {value} while a reference into it is live | Destroying or moving a value would leave an outstanding reference dangling. |
| E499 | cannot borrow a temporary value | `&` needs a named place; temporaries and get/set members have no stable storage to borrow. |

### Example — E494 (escaping reference)

```kestrel
struct Box { var value: Int64 }

func dangling() -> &Int64 {
    let box = Box(value: 42)
    box.value     // error[E494]: cannot return this reference: it borrows
                  // local `box`, which does not outlive the call
}
```

Only parameter-rooted or `Pointer`-derived references can be returned. The
same rule rejects returning a closure that captures a local (the closure's
environment is rooted at its captures).

## E500–E507 — Moves & ownership

These apply to non-`Copyable` types, which move instead of copy.

| Code | Message | Explanation |
|---|---|---|
| E500 | use of moved value '{name}' | The value was consumed (moved) earlier and then used again. |
| E501 | value '{name}' may have been moved | The value is moved on some control-flow paths but not others, then used. |
| E502 | {kind} '{type_name}' has Cloneable field '{field_name}' but does not conform to Cloneable | Containers of `Cloneable` fields must themselves conform to `Cloneable`. |
| E503 | cannot move '{name}' out of a borrowed value | A non-copyable value can't be moved out of a place you only borrow (e.g. a plain `x: T` parameter, or through get/set accessors). |
| E504 *(warning)* | returned reference points into local '{name}', whose storage dies when the function returns | A `Pointer`-derived reference into dead stack storage escapes (unverified pointer territory). |
| E505 | static variable '{name}' has non-Static type '{ty}' | A global lives for the whole program, so its type must be `Static` (reference-free). |
| E506 | cannot move captured value '{name}' out of a closure | A `normal` / `mutating` / `escaping` closure may be called more than once but holds a single non-copyable value, so its body can't move that capture out. **Lifted inside a `consuming` body** — a one-shot closure runs at most once, so moving captures out is exactly what it is for. (A `consuming` body moving a capture the frame only *borrows* is still rejected.) |
| E507 | cannot move / consume / destroy '{name}' while a closure capturing it is live · a closure viewing '{name}' cannot outlive '{name}' | The freeze rule (docs/design/closures.md). While a live `normal` / `mutating` closure carries a view of a place, that place cannot be moved, passed to a `consuming` parameter, or `deinit`ed, and a value carrying the view cannot be stored into a longer-lived binding. Plain reassignment stays legal. The closure analogue of E498. |

### Example — E500 / E501 (use after move)

```kestrel
struct Res: not Copyable { var id: Int64 }

func consume(consuming r: Res) { }

func useTwice(consuming r: Res) {
    consume(r)
    consume(r)      // error[E500]: use of moved value 'r'
}

func maybeUse(consuming r: Res, cond: Bool) {
    if cond { consume(r) }
    consume(r)      // error[E501]: value 'r' may have been moved
}
```

### Example — E503 (move out of borrow)

```kestrel
struct Res: not Copyable { var id: Int64 }

func steal(r: Res) -> Res {   // `r: Res` borrows by default
    r                          // error[E503]: cannot move 'r' out of a borrowed value
}                              // fix: declare the parameter `consuming r: Res`
```

### Example — E506 (move captured value out of closure)

```kestrel
struct Res: not Copyable { var id: Int64 }

func runNormal(f: () -> Res) -> Int64 { f().id }

func capture() {
    let r = Res(id: 7)
    let n = runNormal({ () in r })   // error[E506]: cannot move captured value 'r'
                                     //              out of a closure
}

// The same literal is legal against a `consuming` expected type:
func runOnce(consuming f: consuming () -> Res) -> Int64 { f().id }
```

### Example — E507 (freeze rule)

```kestrel
struct Res: not Copyable { var id: Int64 }

func sink(consuming r: Res) { }

func frozen() {
    let r = Res(id: 1)
    let f = { r.id }   // a normal closure captures a VIEW of `r.id`
    sink(r)            // error[E507]: cannot consume 'r.id' while a closure
                       //              capturing it is live
    f()
}

func outlives() -> Int64 {
    var g: () -> Int64 = { () in 0 }
    if true {
        let r = Res(id: 9)
        g = { r.id }   // error[E507]: a closure viewing 'r.id' cannot outlive 'r.id'
    }                  // `r` dies here; `g` would dangle
    g()
}
```

## E600–E614, E623 — Closures, externs & declaration shape

| Code | Message | Explanation |
|---|---|---|
| E600 | *(reserved — not emitted)* | The check moved into the constraint solver, where an arity-mismatched `it` surfaces as an inference error under **E100**. The code is held so it is not reallocated. |
| E601 | closure has {actual} parameters, but expected {expected} | The closure's parameter count doesn't match the expected function type. |
| E602 | *(reserved — not implemented)* | Closure escape analysis; the descriptor is registered but no emit site exists yet. |
| E603 | cannot assign to captured variable '{name}' | A **normal** closure captures read-only views, so any assignment target rooted at a capture — the bare local or a projection like `c.n = 5` — is rejected. The note points at the fix: give the closure a `mutating` expected type (e.g. `mutating () -> ()`) to write back to the original, or fold the value and return it. Lifted for `mutating` (its views are `&mutating`) and for `consuming` / `escaping` (they own their captures). |
| E604 | cannot assign to closure parameter '{name}' | Closure parameters are immutable. |
| E605 | parameter/return type does not conform to FFISafe | `@extern` signatures may only use FFI-safe types. |
| E606 | could not infer type for closure parameter | The closure needs type context (annotate the parameter or the binding). |
| E607 | subscript must have at least one parameter | Subscripts index by something; zero-parameter subscripts aren't allowed. |
| E608 | subscript must have a body | Subscript declarations outside protocols need an implementation. |
| E609 | @extern functions cannot be generic | Generic functions have no stable ABI to export. |
| E610 | @extern functions cannot have a body | Extern functions are implemented in external code. |
| E611 | @extern function parameter '{name}' must use consuming access mode | Extern functions receive values, not references. |
| E612 | @extern requires a calling convention | Write e.g. `@extern(.C)`. |
| E613 | required parameter '{name}' cannot follow parameter '{default_name}' which has a default value | Defaulted parameters must come last. |
| E614 | default value cannot reference parameter '{name}' | Defaults are evaluated at each call site and can't see other parameters. |
| E623 | function '{name}' requires a body | A non-protocol, non-extern function was declared without a body. |

### Example — E601 (closure arity)

```kestrel
let inc: (Int64) -> Int64 = { (a, b) in a }
// error[E601]: closure has 2 parameters, but expected 1
```

`{ it + 1 }` against a two-parameter type is the same mistake, but it is caught
by the solver and reported as **E100**, not E600.

## E615–E618 — Entry point

An executable build (`kestrel build`, `flock build`/`run`) requires exactly one
`@main` function. `@main` may return `()`, `!`, or any type conforming to
`Exitable` (e.g. `ExitCode`, integer types, `Result[(), E]` for throwing mains).

| Code | Message | Explanation |
|---|---|---|
| E615 | \`@main\` on '{name}' must be a free function | `@main` is only allowed on a free (module-level) function, not a method. |
| E616 | \`@main\` function '{name}' has an invalid return type | The return type must be `()`, `!`, or conform to `Exitable`. |
| E617 | more than one \`@main\` in this build | Two functions are marked `@main`; an executable has exactly one entry point. |
| E618 | no entry point: an executable build requires a \`@main\` function | Building an executable with no `@main` anywhere in the program. |

### Example

```kestrel
@main
func run() {           // OK — exactly one free @main
    println("hello")
}

struct App {
    @main func go() { }   // error[E615]: `@main` must be a free function
}

@main
func other() -> String {  // error[E616]: String does not conform to Exitable
    "done"
}
// a second free `@main` anywhere in the build   → error[E617]
// an executable build with no `@main` at all    → error[E618]
```

## E619–E622 — Place accessors

These govern `get` / `set` / `ref` / `mutating ref` accessor blocks on
computed members.

| Code | Message | Explanation |
|---|---|---|
| E619 | duplicate read provider: this member declares both \`get\` and \`ref\` | Reads need exactly one provider — keep `get` or `ref`, not both. |
| E620 | duplicate write provider: this member declares both \`set\` and \`mutating ref\` | Writes need exactly one provider. |
| E621 | \`ref\` accessors are not allowed in protocols or protocol extensions | Declare `ref` accessors on concrete types only. |
| E622 | this member has a write provider but no read provider | A `set`/`mutating ref` without a `get` or `ref`; add a read provider. |

## E624–E625 — Closure kinds

A function type may name a closure *kind* — `mutating (T) -> U`,
`consuming (T) -> U`, `escaping (T) -> U`; an unmarked type is the *normal*
kind. The kind fixes how the closure holds its captures and how it may be
called — see [memory-model/closures.md](memory-model/closures.md).

| Code | Message | Explanation |
|---|---|---|
| E624 | closure kind mismatch: expected {expected}, found {actual} | The closure value's kind does not pass where the expected kind is required. Only `normal → mutating`, `escaping → normal`, `escaping → consuming`, and the same-kind diagonal are accepted. |
| E624 | an owning closure cannot capture '{name}': it carries a reference | An `escaping` / `consuming` literal snapshots its captures into an environment that may outlive the frame, so it cannot capture a ref binding or any other non-`Static` value. Copy the referenced value into a `let` first and capture that. |
| E625 | a '{kind}' closure parameter must have the '{kind}' access mode | A `mutating`-kind parameter must be declared `mutating`, and a `consuming`-kind parameter `consuming` — the kind dictates the access the callee needs to call it. |

### Example — E624 (passing table)

```kestrel
func takesConsuming(consuming f: consuming () -> Int64) -> Int64 { f() }

func demo(x: Int64) -> Int64 {
    let n: () -> Int64 = { x };
    takesConsuming(n)
    // error[E624]: closure kind mismatch: expected a 'consuming' closure,
    //              found a normal closure
}
```

A normal or `mutating` closure holds *views* of the enclosing frame, so it
owns nothing a `consuming` slot could take and can never reach an `escaping`
slot. A `consuming` closure is one-shot and fits only a `consuming` slot. An
`escaping` closure passes everywhere except `mutating` (its calls are shared,
not exclusive).

### Example — E625 (kind / access-mode pairing)

```kestrel
func each(action: mutating (Int64) -> ()) { }
// error[E625]: a 'mutating' closure parameter must have the 'mutating' access mode

func eachOk(mutating action: mutating (Int64) -> ()) { }   // ok
```

## E700–E707 — String literals & escapes

| Code | Message | Explanation |
|---|---|---|
| E700 | invalid escape sequence \`{seq}\` | Unknown `\x` escape in a string literal. |
| E701 | ASCII escape \`\x{NN}\` is out of range | `\x` escapes must be in `0x00`–`0x7F`. |
| E702 | invalid Unicode escape \`{value}\` | The `\u{...}` value is not a valid Unicode scalar. |
| E703 | incomplete escape sequence at end of string | The string ends in the middle of an escape. |
| E704 | multi-line string content less indented than closing delimiter | Every line must start with at least the closing `"""`'s indentation. |
| E705 | multi-line string opener \`"""\` must be followed by a newline | Content starts on the line after the opening `"""`. |
| E706 | multi-line string closer \`"""\` must be on its own line | Content ends on the line before the closing `"""`. |
| E707 | unterminated string | The string literal is never closed. |

## E800–E809 — Syntax

Emitted by the parser (`kestrel-parser`, `syntax_error.rs`). Every syntax
error carries one of these codes; the message names what was expected and
what was found (`expected \`;\`, found identifier`). A missing `;`, `)` or `}`
is reported on the token *before* the gap, where it belongs.

| Code | Message | Explanation |
|---|---|---|
| E800 | expected {tokens}, found {token} | A specific token was required here (e.g. `:` after a parameter name, `=>` in a match arm). |
| E801 | expected \`;\`, found {token} | A statement that is not `if`/`while`/`for`/`loop`/`match` must end with `;` unless it is the block's final value. |
| E802 | expected \`)\` / \`]\` / \`}\`, found {token} | A bracket was opened and never closed. |
| E803 | expected expression, found {token} | An expression was required (after an operator, as an argument, as a statement). |
| E804 | expected identifier after \`.\` | A `.` must be followed by a member name (or a tuple index). |
| E805 | expected expression after \`throw\` | `throw` needs the error value to throw. |
| E806 | expected declaration / member declaration, found {token} | The token cannot begin a declaration in this position (e.g. `init` at the top level, `deinit` in an extension, a statement outside a function). |
| E807 | expected type, found {token} | A type was required (after `:`, `->`, in `[...]` type arguments). |
| E808 | expected pattern, found {token} | A pattern was required (after `let`, `case`-style match arms, parameters). |
| E809 | expected a name, found {token} | A declaration's name is missing; a keyword in name position (`func case()`) is reported here. |
