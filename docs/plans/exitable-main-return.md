# `Exitable` — Rich `@main` Return Types

**Status**: Implemented (0.17)
**Issue**: [#110](https://github.com/kestrellang/kestrel/issues/110)
**Target**: 0.17
**Depends on**: [#109](https://github.com/kestrellang/kestrel/issues/109) (`@main` attribute)

> **As shipped.** This doc records the design *and* where implementation refined
> it (see Resolved Questions / Alternatives). Key deviations from the first
> draft, decided during build: `ExitCode` stores `lang.i8` (not the `Int8`
> struct); raw `lang.iN` returns are **kept** (back-compat), not dropped; only
> the unit-specialized `Result[(), E]` conforms (the generic
> `Result[T: Exitable, E]` overlaps it and ICEs); `()` / `!` stay wrapper
> special-cases (they cannot be `extend`ed).
>
> **Update (0.17 — conformance specialization).** The last two limitations were
> lifted (see `conformance-specialization.md`): `()` and `!` now **conform** to
> `Exitable` via synthetic `lang.()`/`lang.!` entities, the `Result` conformance
> is now the **generic** `extend Result[T, E]: Exitable where T: Exitable` (the
> overlap ICE is fixed by most-specific-wins), and E616 recurses through `Result`
> on its Ok type. So `main() -> T throws E` works for any `Exitable` `T`
> (`-> Int64 throws E`, `-> ExitCode throws E`, …), not just unit. The `@main`
> wrapper still structurally special-cases a *direct* `-> ()` / `-> !` return
> (`Tuple([])` → exit 0, `Never` → unreachable).

## Summary

Introduce an `Exitable` protocol so a `@main` function can return any type that
knows how to produce a process exit code, generalizing the fixed `Void` /
primitive-integer return shipped in #109. The compiler synthesizes the real C
`main` as a wrapper that calls `report()` on whatever `@main` returns.

```kestrel
@builtin(.Exitable)
public protocol Exitable {
    @builtin(.ExitableReport)
    consuming func report() -> ExitCode
}

@builtin(.ExitCode)
public struct ExitCode {
    private var rawValue: lang.i8
    public init(value: UInt8)            // positional: `ExitCode(2)`
    public static var success: ExitCode  // 0
    public static var failure: ExitCode  // 1
}
```

`@main` may then return `ExitCode`, any `IntN` / `UIntN`, `()`, `!`, a raw
`lang.iN`, or a throwing `main` (`-> () throws E`, i.e. `Result[(), E]` whose
`Err` is `Formattable`):

```kestrel
@main func main() -> ExitCode {
    if ok { ExitCode.success } else { ExitCode(2) }
}

@main func main() -> () throws ConfigError {        // -> Result[(), ConfigError]
    let _ = try Config.load("app.toml");            // .Err → printed to stderr, exit 1
    .Ok(())                                          // success → exit 0
}
```

## Motivation

In 0.16, `@main` (#109) keeps the raw C-ABI entry: it may return `()` or an
internal `lang.iN` primitive only (E616). Stdlib `Int64`, `Result`, custom
types, and a throwing `main` are all rejected. That forces every fallible
program to catch its own errors, format them, and call `exit` by hand.

`Exitable` moves the "what exit code does this value mean?" decision into the
type, the way Rust's `Termination` trait does. A throwing `main` then works for
free, because `T throws E` already desugars to `Result[T, E]` and `Result`
conforms.

## Design

### Decisions locked

These were settled during design (some reverse assumptions in the issue):

1. **Name is `Exitable`**, not `Terminable`. Kestrel's protocol house style is
   `-able` (`Equatable`, `Cloneable`, `Matchable`), and `Exitable` pairs
   directly with `report()` / `ExitCode`.
2. **A method `report() -> ExitCode`, not a computed property `exitCode: Int32`.**
   This *reverses* the issue's "computed property" lock: a computed
   `exitCode: ExitCode` reads badly (`.exitCode` returning an `ExitCode`), and a
   dedicated value type is worth more than a bare integer. Mirrors Rust's
   `fn report(self) -> ExitCode`.
3. **`report()` is `consuming`.** `Result` is `not Copyable`; consuming `self`
   lets the conformance move out the `Ok`/`Err` payload. Harmless for the
   `Copyable` conformers (`ExitCode`, the integers).
4. **`ExitCode` wraps a private `lang.i8`.** (Draft said the `Int8` struct;
   shipped as the raw `lang.i8` primitive so the synthesized wrapper extracts
   the scalar in one `struct_extract` without resolving `Int8`.) Signed `i8` so a
   returned code sign-extends into the C `int` exactly the way `exit(-1)`
   truncates to `255`; only the low 8 bits survive on POSIX (`WEXITSTATUS`), so
   the meaningful range is 0–255 regardless. The byte is private; construct via
   the positional `ExitCode(_:)` (`init(value: UInt8)`)
   or the `.success` / `.failure` constants.
5. **No second `_Exitable` protocol.** The raw-`i8` seam the C wrapper needs is
   `ExitCode`'s backing field, reached structurally in the synthesized wrapper —
   not a separate sealed protocol.
6. **`Void` and `!` are accepted but do *not* conform** — see
   [Semantics → Void and Never](#void-and-never). They cannot be `extend`ed
   today (structural types, no nominal entity), so the wrapper special-cases
   them.
7. **`process.exit(code:)` is out of scope** — already shipped as
   `std.os.exit(code: Int32)` (`lang/std/os/proc.ks:118`), and its signed
   `Int32` is the imperative `exit(-1)` escape hatch.
8. **No Windows / argc-argv in scope.** No Windows support exists yet; arg
   access is separate work.

### The `Exitable` protocol and `ExitCode`

Both live in a new file `lang/std/os/exitable.ks` (`module std.os`,
co-located with `exit`), prelude-exported so `@main -> ExitCode` needs no
explicit import.

```kestrel
/// A type a `@main` function may return: it knows how to produce a process
/// exit code. The compiler synthesizes C `main` as a wrapper that calls
/// `report()` on whatever `@main` returns.
@builtin(.Exitable)
public protocol Exitable {
    @builtin(.ExitableReport)
    consuming func report() -> ExitCode
}

/// A process exit code. 0 conventionally means success; non-zero means
/// failure. Only the low 8 bits survive on POSIX, so the range is 0–255.
public struct ExitCode {
    private var rawValue: lang.i8

    public init(value: UInt8) { self.rawValue = value.raw }

    public static var success: ExitCode { ExitCode(0) }
    public static var failure: ExitCode { ExitCode(1) }
}

extend ExitCode: Exitable {
    consuming func report() -> ExitCode { self }
}
```

## Semantics

### Conformance set

| Type | Exit code | Notes |
|---|---|---|
| `ExitCode` | `self` | identity |
| `Int8/16/32/64` | `ExitCode(UInt8(from: self))` | low 8 bits |
| `UInt8/16/32/64` | `ExitCode(UInt8(from: self))` | `UInt8` uses `self` directly |
| `Result[(), E]` | `Ok`→`0`; `Err(e)`→print `e` to stderr, `.failure` | `where E: Formattable` — **unit-Ok only** (see Throwing `main`) |
| `()` (Void) | `0` | **not a conformer** — wrapper special-case |
| `!` (Never) | n/a | **not a conformer** — wrapper special-case; never returns |
| `lang.iN` | sign-extend to `i64` | **not a conformer** — wrapper handles raw primitives directly (back-compat) |

Integer and `Result` conformances are declared **in `exitable.ks`** as
retroactive `extend`s, not in `lang/std/numeric/` or `lang/std/result/`. This
keeps the dependency edge pointing the natural way (`std.os` imports
`std.numeric`/`std.result`, never the reverse) and leaves the generated
`integer.ks.template` untouched.

```kestrel
extend Int8:  Exitable { consuming func report() -> ExitCode { ExitCode(UInt8(from: self)) } }
// … Int16/Int32/Int64, UInt16/UInt32/UInt64 identical; UInt8 uses ExitCode(self)
```

> **Note — permissive vs. strict.** Blessing every integer width is a
> deliberate, more permissive choice than Rust (which conforms only `ExitCode`,
> `()`, `Result` and makes you opt in via `ExitCode`). It keeps the C-familiar
> `@main -> Int64 { 0 }` working. The cost is that any integer is silently a
> valid exit code; accepted for ergonomics.

### Throwing `main`

A throwing `@main` is written `-> () throws E` (the parser requires an explicit
return type before `throws`; `func main() throws E` does **not** parse). That
desugars to `Result[(), E]` (`lang/std/result/result.ks:415`), covered by the
**unit-specialized** conformance:

```kestrel
extend Result[(), E]: Exitable where E: Formattable {
    consuming func report() -> ExitCode {
        match self {
            .Ok(_)      => ExitCode.success,
            .Err(error) => {
                let _ = eprintln(error);   // std.io.eprintln, takes `some Formattable`
                ExitCode.failure
            }
        }
    }
}
```

(Match arms bind without `let`: `.Err(error)`, not `.Err(let error)`.)

The error bound is **`E: Formattable`**, not `E: Error` — Kestrel has no `Error`
protocol (`lang/std/core/error.ks` defines `Tryable`/`FromResidual` only), and
the real requirement is printability. This is the direct analog of Rust's
`E: Debug`. Errors print via `std.io.eprintln` (`lang/std/io/stdio.ks:168`).

**Why only `Result[(), E]`, not the generic `Result[T: Exitable, E]`?** A spike
showed the two cannot coexist: for `Result[(), _]` the conformance selector
routes `.report()` through the *generic* body (where `().report()` has no
witness, since `()` isn't `Exitable`) → ICE `Callee::Witness not resolved`. The
where-clause `T: Exitable` correctly disqualifies the generic in isolation, but
not when a partial specialization overlaps it. So v1 ships **only** the
unit-specialized conformance. Consequence: `main() -> NonUnit throws E` (e.g.
`-> Int32 throws E` → `Result[Int32, E]`) is **rejected by E616** — the rare
"throw *and* return a value" case is unsupported (workaround: return `ExitCode`
and handle errors internally). Lifting this needs either a fix to overlapping-
conformance selection or making `()` itself `Exitable` (a larger language
change — see [Alternatives](#alternatives-considered)).

### Void and Never

`()` and `!` **cannot conform to a protocol today.** Both are structural types
with no nominal entity: `extend (): P` / `extend !: P` parse but are silently
inert (the conformance never registers, because `ExtensionTargetEntity`
resolves only `AstType::Named` targets — `lib/kestrel-name-res/src/extensions.rs:47`),
and `extend Void: P` / `extend Never: P` hit **E452 "unknown type"** (no such
entity exists). There is no stdlib precedent for extending a tuple, unit, never,
or `lang.iN` primitive.

Rather than make them nominal (a separate language change — see
[Alternatives](#alternatives-considered)), the **synthesized wrapper
special-cases them** by branching on `@main`'s declared return type:

- **`()`** → call `run()`, then `return 0`.
- **`!`** → call `run()` (it never returns); terminator `Unreachable`.
- raw **`lang.iN`** → `run()`, then sign-extend the result to `i64` (back-compat).
- **`T: Exitable`** → `return run().report().rawValue` (sign-extended).

### Entry-point wrapper

The user's `@main` function is demoted to an ordinary function (call it `run`,
`is_main = false`). The compiler synthesizes a new function, marked
`is_main = true` and exported as C `main`, returning `lang.i64`:

```
func <synthesized main>() -> lang.i64 {
    __kestrel_init_statics();          // moved out of the user function
    // dispatch on run()'s return type:
    //   T: Exitable  →  let code = run().report();  sextend(code.rawValue, i64)
    //   lang.iN      →  let v = run();  sextend(v, i64)  (i64 returned as-is)
    //   ()           →  run();  iconst 0
    //   !            →  run();  unreachable
}
```

Static initialization (previously prepended into the user `@main`'s entry block
by `inject_init_call_into_main`) moves into the wrapper's first statement; that
function is replaced by `synthesize_main_wrapper`
(`lib/kestrel-mir-lower/src/items/static_lower.rs`).

Reading `code.rawValue` (a `private` field) is sound here: the wrapper is
synthesized MIR, where source-level privacy does not apply — the same way
deinit/clone/drop shims access fields directly. `ExitCode`'s field is a raw
`lang.i8`, so `struct_extract` yields the scalar directly, then `IntWiden`
sign-extends to `i64`. One subtlety the implementation hit: `Op1`/`IntWiden` is a
*non-consuming* read, so the `@owned` source scalar needs an explicit
`DestroyValue` (a no-op the expand pass drops) to satisfy OSSA verification.

## Desugaring / synthesis

The wrapper is synthesized in **MIR-lowering Phase 3**, alongside static-init
synthesis in `static_lower.rs`, *pre-monomorphization* — so the `run()` call and
the `report()` witness call go through mono and expand like any other body.

- **Scaffolding template:** `synthesize_master_init`
  (`static_lower.rs:125`) — allocates a synthetic entity, registers a name,
  builds a `FunctionDef { kind: Free }`, hand-constructs an `OssaBody` (alloc
  block, push `InstKind::Call`, literal, `Terminator::Return`).
- **Method-dispatch template:** `drop_shim` / `clone_shim`
  (`lib/kestrel-mir/src/passes/drop_shim.rs`) — precedent for a synthesized body
  that *calls a user method* on a concrete type.
- **`run()` call:** `Callee::direct(run_entity)`.
- **`report()` call:** `Callee::Witness { protocol, method, self_type, .. }`
  (see `lib/kestrel-mir-lower/src/body/call/mod.rs:341`), resolved to a concrete
  impl during mono via `resolved_witnesses`.

## Pipeline Trace

| Stage | What happens | Changes needed |
|-------|-------------|----------------|
| **Builtin registry** | `Exitable` / `report` resolvable by entity | +2 `Builtin` variants + 3 match arms in `lib/kestrel-hir/src/builtin.rs` |
| **Stdlib** | Protocol, `ExitCode`, conformances | new `lang/std/os/exitable.ks`; prelude export |
| **Entry analyzer (E616)** | Return type must be `()` / `!` / `Exitable` | rewrite `main_return_type_ok` (`entry_point.rs:161`) to query `ConformingProtocols` |
| **MIR lowering** | Synthesize C-`main` wrapper; demote user `@main` to `run`; move static init in | new wrapper synthesis in `static_lower.rs`; flip `is_main` |
| **Monomorphization** | Wrapper's `run()` + `report()` calls instantiate | none (rides existing call lowering) |
| **Codegen** | Wrapper genuinely returns `i64`; exported as `main` | **remove** `is_main` I64-forcing (`abi.rs:33`) and byte-extraction (`terminator.rs:120`); keep export-as-`main` (`context.rs:288`) |

The codegen change is a net *simplification*: once the wrapper returns a real
`i64`, the `is_main` special-cases in `return_mode` and `compile_return` are
dead and can be deleted.

## Diagnostics

| ID | Name | Change |
|----|------|--------|
| `E616` | `invalid_main_return_type` | Rewrite (`main_return_type_ok`): accept iff the return type is `()`, `!`, a `Result` with **unit** Ok (`main() throws E`), a raw `lang.iN` (kept for back-compat), or a `Named` type conforming to `Exitable` (`is_lang_primitive_int(e) \|\| conforms_to_exitable(e)`). Message: "`@main` must return `()`, `!`, or a type conforming to `Exitable`". |

E615 (`main_not_free_function`), E617 (`multiple_main`), E618 (`missing_main`)
are unchanged.

## Builtin registration

Adding the builtin is data-driven — the only compiler file is
`lib/kestrel-hir/src/builtin.rs` (Rust exhaustiveness on `name()`/`kind()` forces
the arms):

```rust
// enum Builtin (~line 252, by FormattableFormatIntoMethod)
Exitable,
ExitableReport,

// name()            → "Exitable" (source name) / "ExitableReport" (attr name)
// from_attribute_name() → "Exitable" => Exitable, "ExitableReport" => ExitableReport
// kind()            → Exitable: BuiltinKind::protocol(); ExitableReport: BuiltinKind::ProtocolMethod
```

Name-res (`ResolveBuiltin` / `EntityBuiltin` / `BuiltinIndex`,
`lib/kestrel-name-res/src/resolve_builtin.rs`) is variant-generic; no other file
changes. The compiler obtains the protocol entity via
`ResolveBuiltin { builtin: Builtin::Exitable }` and emits the `report()` call as
a witness call (pattern: operator desugaring in
`lib/kestrel-hir-lower/src/desugar.rs`).

## Implementation plan

1. **Builtin variants** — `lib/kestrel-hir/src/builtin.rs`: add `Exitable` /
   `ExitableReport` + the three arms.
2. **Stdlib** — new `lang/std/os/exitable.ks`: `Exitable`, `ExitCode`, the
   `ExitCode`/integer/`Result` conformances; prelude-export `Exitable` +
   `ExitCode`.
3. **E616** — `lib/kestrel-analyze/src/compilation/entry_point.rs`: replace
   `is_lang_primitive_int` check with an `Exitable`-conformance query; add the
   shared `()` / `!` / `Exitable` predicate.
4. **Wrapper synthesis** — `lib/kestrel-mir-lower/src/items/static_lower.rs`:
   synthesize the C-`main` wrapper (return-type trichotomy), move static init
   into it, flip `is_main` from user `@main` (→ `run`) to the wrapper.
5. **Codegen cleanup** — delete the `is_main` branches in
   `lib/kestrel-codegen-cranelift/src/abi.rs:33` (`return_mode`) and
   `terminator.rs:120` (`compile_return`); keep the export-as-`main` in
   `context.rs`.
6. **Tests** — execution tests: `@main` returning `ExitCode.success`/`.failure`,
   `ExitCode(value:)`, each `IntN`/`UIntN`, `()`, throwing-`main` success and
   `.Err` (asserting stderr + non-zero exit), and a custom `Exitable` struct.

## Resolved Questions

1. **`Void` conformance?** No — `()` is structural and unextendable; the wrapper
   special-cases it to `0`. (Resolves the issue's open question.)
2. **`!` conformance?** No — same as `Void`; wrapper special-cases (never
   returns).
3. **Error bound for throwing `main`?** `E: Formattable` (printability), not
   `E: Error` — there is no `Error` protocol; matches Rust's `E: Debug`.
4. **Computed property vs method?** Method `report() -> ExitCode`, reversing the
   issue's lock — the value type beats a bare `Int32`.
5. **`ExitCode` backing width?** `lang.i8` (the raw primitive). Sign-extends to
   the C `int` like C's own truncation; only 8 bits are portable anyway.
6. **`.success` / `.failure`?** Yes — static constants (0 / 1), the ergonomic
   payoff of the value type.
7. **Where do conformances live?** In `exitable.ks` (retroactive), to keep
   `std.numeric`/`std.result` from depending upward and avoid editing the
   generated integer template.

## Open Questions

None blocking. One deliberately deferred: see Alternatives — making `()` / `!`
genuinely `Exitable` (nominal + extendable) instead of wrapper special-cases.

## Alternatives considered

- **Make `Void` / `Never` extendable.** Seed nominal entities for `()` and `!`
  in `lang_module.rs` and teach `ExtensionTargetEntity` to resolve
  `AstType::Unit`/`AstType::Never` to them (both gates must change in lockstep,
  mirroring how `Int64` wraps `lang.i64`). Then `()` / `!` conform like any type
  and the wrapper needs no special-case. Rejected for 0.17 as a larger,
  orthogonal language change; the two-case wrapper branch is cheap and contained.
- **Two-layer `_Exitable` (sealed, `i8`) + public `Exitable`.** Considered for
  ABI sealing. Rejected: the raw-`i8` seam is already `ExitCode`'s backing field,
  reached in the wrapper — a second protocol adds indirection with no payoff.
- **Bare-integer protocol (`report() -> Int32`, no `ExitCode`).** Simpler, but
  loses `.success`/`.failure`, the 0–255 self-documentation, and a home for
  platform quirks. Rejected.
- **Drop raw `lang.iN` returns.** Rejected — ~hundreds of existing tests use
  `@main func main() -> lang.i64`, so the wrapper keeps handling `lang.iN`
  directly (sign-extend, no `report()`), preserving #109 behavior alongside the
  new `Exitable` path.
- **Generic `Result[T: Exitable, E]` + specialized `Result[(), E]`.** Rejected —
  the two overlap and ICE (`Callee::Witness not resolved`); see Throwing `main`.
  Ship the unit-specialized conformance only.
