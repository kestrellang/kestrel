# Kestrel Compiler Error Codes

> Generated from the compiler source (diagnostic descriptors in `lib/`) on
> 2026-07-01. Every code below corresponds to a descriptor or emit site in the
> compiler; message templates use `{name}`-style placeholders for interpolated
> values. Unless marked *(warning)*, a code is an error. A few code numbers are
> shared by two unrelated diagnostics; both meanings are listed.

Most diagnostics print as `message [E-code]` with source labels. Inference
errors are all surfaced under the umbrella code **E100**; the detailed message
comes from the type checker.

## Contents

- [E001–E009 — Control flow & definite initialization](#e001e009--control-flow--definite-initialization)
- [E100–E121 — Type checking, parameters & literals](#e100e121--type-checking-parameters--literals)
- [E200–E212 — Mutability, access modes & assignment](#e200e212--mutability-access-modes--assignment)
- [E300–E316 — Patterns & exhaustiveness](#e300e316--patterns--exhaustiveness)
- [E411–E466 — Declarations, generics & protocol conformance](#e411e466--declarations-generics--protocol-conformance)
- [E488–E499 — References & escape checking](#e488e499--references--escape-checking)
- [E500–E506 — Moves & ownership](#e500e506--moves--ownership)
- [E600–E614 — Closures, externs & declaration shape](#e600e614--closures-externs--declaration-shape)
- [E615–E618 — Entry point](#e615e618--entry-point)
- [E619–E622 — Place accessors](#e619e622--place-accessors)
- [E700–E707 — String literals & escapes](#e700e707--string-literals--escapes)

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

## E200–E212 — Mutability, access modes & assignment

| Code | Message | Explanation |
|---|---|---|
| E200 | cannot assign to immutable variable '{name}' | Assignment to a `let` binding; declare it `var` to mutate. |
| E201 | cannot assign to immutable field '{name}' | Assignment to a `let` field, or to a field without a setter. |
| E202 | cannot assign to this expression | The left-hand side of the assignment is not an assignable place (e.g. a call result or literal). |
| E203 | cannot pass immutable binding '{name}' to 'mutating' parameter | A `let` binding can't be handed to a parameter that mutates it. |
| E204 | cannot pass immutable field '{name}' to 'mutating' parameter | An immutable field can't be passed where mutation is required. |
| E205 | cannot pass temporary value to 'mutating' parameter | A temporary (rvalue) has no place to write the mutation back to. |
| E206 | *(declared; not currently emitted)* | Reserved: passing a `let` binding to a `consuming` parameter. |
| E207 | cannot call a 'mutating' method through a shared reference | A `&T` reference is read-only; mutating calls need `&mutating` or an owned `var`. |
| E208 | cannot assign through a shared reference | Writing through `&T` is not allowed; take `&mutating` instead. |
| E209 | a ref binding must be a simple \`let\` | A borrow initializer (`let r = &x`) can't be combined with `var` or a destructuring pattern. |
| E210 | cannot take a \`&mutating\` borrow of immutable variable '{name}' | Mutable borrows need a mutable place (`var`, mutable field, or `Pointer.mutatingValue`). |
| E211 | \`&\` pattern bindings are not supported in this position | `&` patterns are only allowed in match-arm patterns, not in `let`/`for`/conditions/params. |
| E212 | closure cannot capture non-Static binding '{name}' | A closure environment may outlive the scope, so only Static (reference-free) values can be captured; ref bindings can't be captured at all. |

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

## E411–E466 — Declarations, generics & protocol conformance

| Code | Message | Explanation |
|---|---|---|
| E411 | duplicate method '{name}': defined on both the type and an extension | An extension redefines a method the type already declares. |
| E412 | duplicate method '{name}' in extensions of '{type}' | Two extensions of the same type instantiation define the same method. |
| E413 | computed properties must use 'var' | A computed property (with `get`/`set`) can't be declared `let`. |
| E415 | enums cannot have stored fields | Enums carry data in case payloads, not stored fields. |
| E416 | static stored properties not supported in generic types | A generic type would need one global per instantiation; use a computed static instead. |
| E417 | 'static' is redundant here | Global (module-level) properties are already static. |
| E417 | protocol method '{method}' in '{protocol}' cannot have a body | Protocol requirements are signatures only; put default bodies in a protocol extension. |
| E418 | '{name}' cannot be static in this context | `static` is not allowed at module level. |
| E419 | @builtin(.{feature}) must be a marker protocol (no required methods or types) | Builtin language-feature protocols cannot declare requirements. |
| E420 | fields of '{type_name}' do not conform to '{protocol}' | A field-checked conformance (e.g. `FFISafe`) requires every stored field to conform too. |
| E421 | '{type_name}' conforms to '{child_protocol}' but not its parent '{parent_protocol}' | Conforming to a child protocol requires conforming to the protocols it inherits from. |
| E422 | enum '{enum_name}' cannot conform to protocol '{protocol_name}' | This protocol is restricted to non-enum types. |
| E423 | cannot conform to \`{protocol_name}\` and opt out of \`Copyable\` | A conformance that requires `Copyable` conflicts with `: not Copyable`. |
| E423 | struct \`{name}\` already has a deinit | A type may declare at most one `deinit`. |
| E424 | duplicate definition of {kind} '{name}' | Two declarations of the same kind share a name in the same scope. |
| E424 | '{name}' is not a language feature protocol | `not P` (negative conformance) is only allowed for language-feature protocols like `Copyable`. |
| E425 | '{name}' is already defined as a {original_kind} | A name is reused by a declaration of a different kind (e.g. struct vs func). |
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
| E436 | cannot find type '{name}' in this scope | A type annotation names a type that doesn't resolve. |
| E437 | undeclared type parameter '{name}' in where clause | The `where` clause constrains a name that isn't a type parameter in scope. |
| E438 | too few/too many type arguments for '{name}': expected {n}, got {m} | Wrong number of generic arguments (also: the type doesn't accept arguments at all). |
| E439 | type parameter '{name}' shadows outer type parameter | A nested declaration reuses an enclosing generic parameter's name. |
| E440 | no associated type '{name}' on '{type}' | A `where` clause projects an associated type the bound protocol doesn't declare. |
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
| E459 | method '{name}' has wrong receiver kind for protocol '{proto}' | The witness's receiver (`mutating`/`consuming`/plain) doesn't match the requirement. |
| E459 | circular protocol inheritance: '{A}' -> ... -> '{A}' | Protocols inherit from each other in a cycle. |
| E460 | property '{name}' requires a setter to satisfy protocol '{proto}' | The protocol requires `get set` but the witness property is read-only. |
| E461 *(warning)* | unknown attribute '{name}' | An `@attribute` isn't recognized by the compiler. |
| E462 | conflicting associated type '{name}' inherited by protocol '{proto}' | Two inherited protocols declare incompatible versions of the same associated type. |
| E463 | method '{name}' is ambiguous: satisfies requirements of multiple protocols ({list}) | One method would witness several unrelated protocols; implement each conformance in its own `extend Type: Protocol` block. |
| E464 | init has wrong effect for protocol '{proto}' | A witness `init`'s failability/throwing effect doesn't match the protocol requirement. |
| E465 | indirect enums are not yet supported | `indirect` is recognized but not implemented in this version. |
| E466 | 'some' (opaque type) is not allowed in a field type | Opaque `some P` types can only appear in return position. |

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

## E488–E499 — References & escape checking

References (`&T` / `&mutating T`) are second-class: they can't be stored, and
a returned reference must outlive the call. The escape checker (E494–E498)
runs on MIR and tracks each reference's *root provenance* — including
references and closures laundered through structs, enums, and tuples.

| Code | Message | Explanation |
|---|---|---|
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

## E500–E506 — Moves & ownership

These apply to non-`Copyable` types, which move instead of copy.

| Code | Message | Explanation |
|---|---|---|
| E500 | use of moved value '{name}' | The value was consumed (moved) earlier and then used again. |
| E501 | value '{name}' may have been moved | The value is moved on some control-flow paths but not others, then used. |
| E502 | {kind} '{type_name}' has Cloneable field '{field_name}' but does not conform to Cloneable | Containers of `Cloneable` fields must themselves conform to `Cloneable`. |
| E503 | cannot move '{name}' out of a borrowed value | A non-copyable value can't be moved out of a place you only borrow (e.g. a plain `x: T` parameter, or through get/set accessors). |
| E504 *(warning)* | returned reference points into local '{name}', whose storage dies when the function returns | A `Pointer`-derived reference into dead stack storage escapes (unverified pointer territory). |
| E505 | static variable '{name}' has non-Static type '{ty}' | A global lives for the whole program, so its type must be `Static` (reference-free). |
| E506 | cannot move captured value '{name}' out of a closure | A closure only borrows its captures; a non-copyable capture can't be moved out of the closure body. |

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

func capture(consuming r: Res) -> () -> Res {
    { r }   // error[E506]: cannot move captured value 'r' out of a closure
}
```

## E600–E614 — Closures, externs & declaration shape

| Code | Message | Explanation |
|---|---|---|
| E600 | implicit 'it' parameter used in closure expecting {n} parameters | `it` only works when the closure takes exactly one parameter. |
| E601 | closure has {actual} parameters, but expected {expected} | The closure's parameter count doesn't match the expected function type. |
| E602 | closure parameter type mismatch at position {index} | An annotated closure parameter conflicts with the expected function type. |
| E603 | cannot assign to captured variable '{name}' | Captured variables are immutable inside closures. |
| E604 | cannot assign to closure parameter '{name}' | Closure parameters are immutable. |
| E605 | *(declared; superseded)* capturing closure escape | The old syntactic escape check; capturing-closure escapes are now reported as E494 by the MIR escape checker. |
| E605 | parameter/return type does not conform to FFISafe | `@extern` signatures may only use FFI-safe types. |
| E606 | could not infer type for closure parameter | The closure needs type context (annotate the parameter or the binding). |
| E606 | function '{name}' requires a body | A non-protocol, non-extern function was declared without a body. |
| E607 | subscript must have at least one parameter | Subscripts index by something; zero-parameter subscripts aren't allowed. |
| E608 | subscript must have a body | Subscript declarations outside protocols need an implementation. |
| E609 | @extern functions cannot be generic | Generic functions have no stable ABI to export. |
| E610 | @extern functions cannot have a body | Extern functions are implemented in external code. |
| E611 | @extern function parameter '{name}' must use consuming access mode | Extern functions receive values, not references. |
| E612 | @extern requires a calling convention | Write e.g. `@extern(.C)`. |
| E613 | required parameter '{name}' cannot follow parameter '{default_name}' which has a default value | Defaulted parameters must come last. |
| E614 | default value cannot reference parameter '{name}' | Defaults are evaluated at each call site and can't see other parameters. |

### Example — E600 / E601 (closure arity)

```kestrel
let add: (Int64, Int64) -> Int64 = { it + 1 }
// error[E600]: implicit 'it' parameter used in closure expecting 2 parameters

let inc: (Int64) -> Int64 = { (a, b) in a }
// error[E601]: closure has 2 parameters, but expected 1
```

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
