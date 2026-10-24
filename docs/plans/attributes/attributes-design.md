# Attributes and Derives Design

> **Status: design, not started.** Issues
> [#32](https://github.com/kestrellang/kestrel/issues/32) (attribute
> infrastructure), [#34](https://github.com/kestrellang/kestrel/issues/34)
> (built-in attributes), [#111](https://github.com/kestrellang/kestrel/issues/111)
> (`@inline`) and [#33](https://github.com/kestrellang/kestrel/issues/33)
> (derived protocols), milestone 0.18. Base commit `def25459` on
> `future/0.18`. File and line anchors refer to that tree.
>
> Decisions marked **John decides** are recommendations that need sign-off
> before implementation. Everything else is proposed as settled; push back on
> any of it.

## Overview

0.18 makes attributes a real language feature, and builds derived protocol
conformances on top of it. The work has three parts:

1. **Attribute infrastructure.** One registry declares every attribute: its
   name, where it may appear, and the shape of its arguments. One validator
   checks every use against the registry. Consumers read **typed** attribute
   values, never raw strings. This replaces today's arrangement, in which the
   attribute list, the argument parsing and the validation are spread over six
   crates.
2. **Built-in attributes.** `@inline` (the #111 design, adopted unchanged) and
   `@deprecated`, plus the existing `@main`, `@extern`, `@builtin`,
   `@fileconstant` and `@platform`, re-expressed through the registry.
3. **Derives.** `@derive(Equatable, Hashable)` on a struct or enum gives it a
   compiler-built conformance. **What can be derived is defined in Kestrel,
   not in the compiler.** A protocol becomes derivable when its module
   provides a *derivation*: a protocol extension marked `@derivation`, written
   against a small structural API (`Derive.each`, `Derive.all`, ...). The
   stdlib uses this mechanism for `Equatable`, `Hashable`, `Comparable`,
   `Formattable` and `Defaultable`. A user protocol can be made derivable the
   same way.

```kestrel
@derive(Equatable, Hashable, Comparable, Formattable)
struct Version {
    var major: Int64
    var minor: Int64
    var patch: Int64
}

@derive(Equatable, Hashable)
enum Shape {
    case Circle(radius: Float64)
    case Rect(width: Float64, height: Float64)
}

@derive(Equatable)                       // Box[T]: Equatable where T: Equatable
struct Box[T] { var value: T }

@inline(.Always)
func fastPath(x: Int64) -> Int64 { x &+ 1 }

@deprecated("use 'parse(_:)'", since: "0.18", replacement: "parse")
func legacyParse(s: String) -> Int64 { ... }
```

## Current state

What the compiler does at `def25459`:

- **Grammar.** `Attribute = '@' 'ident' AttributeArgs?`
  (`kestrel-syntax-tree/kestrel.ungram:167-179`,
  `kestrel-parser/src/grammar/attrs.rs`). An argument is an optional
  `label:` followed by a string, float, integer or boolean literal, an
  implicit member `.Name`, or a dotted path. Attributes are accepted on
  protocols, structs, enums, enum cases, functions, initializers, subscripts,
  fields and type aliases. They are **not** accepted on `extend`, `deinit`,
  `import`, `module` or parameters (`items.rs:141` rejects a head before a
  headless keyword).
- **Storage.** The AST builder stores
  `Attributes(Vec<AstAttribute>)` on the declaration entity
  (`kestrel-ast-builder/src/components.rs:458`). An argument is
  `AstAttributeArg { label, value: String }`, where `value` is the **raw
  source text**: a string argument still has its quotes, and an implicit
  member still has its leading dot. Only the whole attribute has a span;
  arguments have none.
- **Known names.** A hand-maintained list in the unknown-attribute analyzer
  (`kestrel-analyze/src/compilation/unknown_attribute.rs:42`): `builtin`,
  `dummy`, `extern`, `fileconstant`, `main`, `platform`. Anything else is the
  **warning** E461.
- **Consumers.** Each one matches the name as a string and re-parses the raw
  text itself:

| Attribute | Consumer | How it reads the arguments |
|---|---|---|
| `@main` | `kestrel-analyze/src/compilation/entry_point.rs:134`, `kestrel-mir-lower/src/items/function_sig.rs:81` | Name only |
| `@extern(.C, mangleName: "x")` | `function_sig.rs:74,137`, `body/call/args.rs:349`, `decl/extern_ffi_safe.rs:81`, `decl/function_body.rs:67` | `trim_matches('"')` on `mangleName`. The ABI is never checked to be `.C` |
| `@builtin(.Name)` | `kestrel-name-res/src/resolve_builtin.rs:41` (`EntityBuiltin` query), `decl/duplicate_builtin.rs:142` | `strip_prefix('.')`, then `Builtin::from_attribute_name` |
| `@fileconstant("path")` | `kestrel-mir-lower/src/items/static_lower.rs:423` | `strip_prefix('"')` / `strip_suffix('"')` |
| `@platform(.os)` | `kestrel-ast-builder/src/build.rs:137` | Walks the **syntax tree** before entities exist, because filtering happens during the build |
| `@dummy` | none | A test placeholder |

- **Diagnostics already allocated** to attribute misuse: E400 (duplicate
  `@builtin`), E605 and E609–E612 (`@extern`), E615–E618 (`@main`), E461
  (unknown attribute).
- **Derives.** None. `Cloneable` is already inferred and synthesized as a
  memberwise clone by the copy-classification fold
  (`docs/memory-model/cloneable.md`, "Derived Cloneable Is Automatic";
  `kestrel-mir/src/passes/clone_shim.rs`).
- **Conditional conformance through `where`-gated extensions already
  works.** The stdlib depends on it (`optional.ks:548`
  `extend Optional[T]: Equatable where T: Equatable`, plus `Hashable`,
  `Comparable`, `Formattable` and the `Array`/`Set`/`Dictionary` `Hashable`
  conformances, tested by `testdata/stdlib/array/array_hashable.ks`). The
  bound-aware check is tested by
  `testdata/declarations/extensions/conditional_conformance_bound_rejected.ks`.
  Derives use this mechanism (§C3).

The stringly-typed consumers are the reason #32 exists. A typo like
`@inlne` is only a warning and then does nothing. An `@extern(.Rust)` is
accepted and silently compiled as C. Argument errors have no span to point at.

## Part A: Attribute infrastructure

### A1. Syntax changes

The attribute grammar itself stays. Three changes:

1. **Attributes on `extend`.** `ExtensionDeclaration = AttributeList? 'extend' ...`.
   `@derive` (§C2), `@derivation` (§C4), `@platform` and `@deprecated` all
   need it. The parser's headless-keyword rule (`items.rs:141`) allows an
   attribute head (but not a visibility head) before `extend`.
2. **Attributes on `deinit`.** `DeinitDeclaration = AttributeList? 'deinit' FunctionBody`.
   Only `@inline` and `@platform` apply. This is added for uniformity, so
   "every declaration that has a body takes attributes" is true.
3. **Per-argument spans.** `AstAttributeArg` gains a `span` (and the label
   gets its own span), so diagnostics can point at the argument at fault.

Not in 0.18: attributes on parameters, statements, expressions, `import`
and `module` declarations. The registry (§A2) has a target kind for each
place an attribute can appear, so adding one later is a grammar change and
one enum variant.

### A2. The registry

A single table, `kestrel-ast-builder/src/attrs/registry.rs`, is the **only**
place an attribute is declared. Every other part of the compiler reads it:

```rust
pub struct AttrDef {
    pub name: &'static str,
    pub targets: TargetSet,          // where it may appear (A3)
    pub params: &'static [ParamDef], // argument schema (A4)
    pub repeatable: bool,            // may appear more than once on a declaration
    pub phase: AttrPhase,            // when it takes effect (A6)
    pub conflicts: &'static [&'static str],
    pub doc: &'static str,           // LSP hover and completion text
}
```

- **Why in the AST builder.** `@platform` is evaluated while the builder runs,
  before entities exist. Every later crate already depends on the builder.
  #111 already places attribute processing at the AST stage.
- **Why Rust and not Kestrel source.** Built-in attributes drive compiler
  behaviour, so the compiler has to know them. User-declared attributes come
  with 2.0 metaprogramming (`ROADMAP.md`, "2.0: Metaprogramming"). The
  registry's shape (named, typed, labelled parameters with defaults) is
  chosen so a future `attribute name(params)` declaration in Kestrel maps
  onto it one to one.
- **`KNOWN_ATTRIBUTES` is deleted.** The unknown-attribute check becomes "not
  in the registry".

### A3. Targets

```rust
pub enum AttrTarget {
    Struct, Enum, Protocol, EnumCase, TypeAlias, AssociatedType,
    FreeFunction, Method, Requirement,          // a protocol requirement without a body
    Initializer, Deinit, Subscript,
    StoredField, ComputedProperty, ModuleValue, // module-level `let`/`var`
    Extension, ProtocolExtension,
}
```

- **Derived from the entity.** The target is computed from the entity's
  `NodeKind` plus its parent's kind and whether it has a body. There is one
  function for this, `attr_target(entity)`, next to the registry.
- **Requirements are separate** from methods, because `@inline` needs a body
  and `@builtin` is used on requirements (`format.ks:383`).
- **The diagnostic lists the allowed targets** in words: "'@inline' cannot be
  applied to a struct; it applies to functions, initializers, subscripts and
  computed properties".

### A4. Argument schema and typed values

Arguments are parsed into typed values once, by one function, and stored that
way. The raw text is never seen by consumers again.

```rust
pub enum AttrValue {
    Str(String),               // unescaped, no quotes
    Int(i128),
    Float(f64),
    Bool(bool),
    Case(String),              // `.Always`: the name, no dot
    Path(Vec<String>),         // `Equatable`, `std.fmt.Formattable`
}

pub struct ParamDef {
    pub label: Option<&'static str>,   // None = positional
    pub kind: ParamKind,
    pub required: bool,
    pub variadic: bool,                // positional only, last
}

pub enum ParamKind {
    Str, Int, Bool,
    Cases(&'static [&'static str]),    // a closed set, matched by name (#111)
    OpenCase(fn(&str) -> bool),        // a set owned elsewhere, e.g. `Builtin::from_attribute_name`
    Path,                              // resolved by name resolution later (§C2)
    Any,                               // `@dummy` only
}
```

**Matching rules.** These are the same rules as function arguments, so they
need no new explanation:

- Positional arguments come first, then labelled ones.
- Labelled arguments may come in any order.
- A missing required argument, an unknown label, a repeated label, a
  positional argument after a labelled one, and a value of the wrong kind
  are each errors (§Error cases).
- **Closed case sets are matched by name.** `.Always` is not resolved against
  a real enum. This is #111's "the enum is conceptual" rule, made general:
  the registry holds the case list, and the error names the allowed cases.
  The stdlib documents each case set as an ordinary enum (`InlineMode`,
  `CallingConvention`, `Platform`) for the docs and the LSP. Those enums are
  not referenced by the compiler.
- **Values are literals only.** No expressions and no constants. Constant
  evaluation arrives with `const` (Release Candidate).

### A5. One query, typed results

```rust
/// Typed attributes of one declaration. Pure function of its `Attributes`
/// component, so the incremental engine invalidates it exactly when the
/// attribute text changes.
pub struct AttrsOf { pub entity: Entity }
impl QueryFn for AttrsOf { type Output = Arc<ParsedAttrs>; }

pub struct ParsedAttrs {
    pub attrs: Vec<ParsedAttr>,   // the well-formed ones, in source order
    pub errors: Vec<AttrError>,   // every validation failure, with spans
}

pub enum ParsedAttr {
    Main,
    Extern { abi: Abi, mangle_name: Option<String> },
    Builtin(Builtin),
    FileConstant { path: String },
    Platform(Vec<Os>),
    Inline(InlineMode),
    Deprecated(Deprecation),
    Derive(Vec<DeriveTarget>),    // unresolved paths + spans (§C2)
    Derivation,
    Default,                       // `@default` on an enum case (§C6)
    Dummy,
}
```

- **Single source of truth.** The `Attributes` component (raw syntax plus
  spans) stays as the input. Everything typed is derived from it by `AttrsOf`.
  Nothing else re-parses attribute text. This follows `EntityBuiltin`
  (`resolve_builtin.rs:33`), which already works this way for one attribute.
- **Accessors.** `attrs::get::<Inline>(ctx, entity) -> Option<InlineMode>`
  and `attrs::has::<Main>(ctx, entity)`. The current call sites (the table
  under [Current state](#current-state)) are rewritten to use them.
- **Errors are reported in one place.** `AttrsOf` collects errors but never
  reports them. A single `AttributeValidation` analyzer reports
  `ParsedAttrs::errors` for every entity. It replaces `unknown_attribute.rs`
  and takes over the argument checks the per-attribute analyzers do today.
  Malformed attributes are left out of `attrs`, so consumers never see them.
- **Kept codes.** Checks that are about meaning rather than shape stay where
  they are, with their codes: E400 (duplicate `@builtin` across the program),
  E605 and E609–E611 (FFI rules for `@extern`), E615–E618 (`@main` rules). E612
  ("`@extern` requires a calling convention") becomes the registry's
  missing-argument error for `@extern`. The registry carries an optional
  per-parameter code, so E612 keeps its number and existing tests keep
  matching.

### A6. Phases

Every attribute declares when it takes effect. This decides which crate may
read it, and keeps a consumer from using an attribute before the information
it needs exists.

| Phase | Attributes | Read by |
|---|---|---|
| `Build` | `@platform` | `kestrel-ast-builder` while building, from the syntax node, using the same schema and parser as `AttrsOf` |
| `NameRes` | `@builtin`, `@derive`, `@derivation`, `@default` | `kestrel-name-res` (builtin map, derive targets, derived extensions) |
| `Semantic` | `@main`, `@deprecated` | `kestrel-analyze`, `kestrel-type-infer` |
| `Lowering` | `@extern`, `@fileconstant`, `@inline` | `kestrel-mir-lower`, then MIR, mono and codegen |

**How attributes get past HIR.** HIR holds bodies. Declarations stay in the
ECS, and HIR never carried attributes. Issue #32's "propagated through
AST/HIR/MIR" is met like this:

- Each attribute that changes code generation becomes a **typed MIR field**
  at lowering. Examples are `FunctionDef::inline: Option<InlineMode>` and the
  existing `extern_info` and `FileConstantData`.
- MIR, mono and codegen read those fields, never attributes.
- So there is no second attribute list that could drift from the first.
- **Mono instances** copy the field from their generic definition. Witness
  and closure thunks do not inherit it (§B1).

### A7. Duplicates and conflicts

- A non-`repeatable` attribute appearing twice is an error, and the second
  occurrence is ignored.
- **`@platform` is not repeatable.** Its argument is variadic and means
  *any of*: `@platform(.linux, .darwin)`. Today, `@platform(.linux)
  @platform(.darwin)` means "all of", which can never match. Rejecting the
  repetition removes that trap.
- **`@derive` is repeatable**, and the lists are merged.
- Conflicts are declared in the registry and checked once:

| Pair | Why |
|---|---|
| `@inline` + `@extern` | An extern has no body to inline |
| `@main` + `@extern` | The entry point must be a Kestrel function |
| `@derive(P)` + a declared `P` conformance on the same type | Two conformances (§C7) |
| `@derivation` + any target other than a protocol extension | §C4 |

### A8. Tooling

- **LSP.** Completion after `@` lists the registry entries allowed at the
  cursor's target, with `doc` as detail. Completion inside the parentheses
  offers labels and, for case parameters, the case list. Hover on an
  attribute shows its `doc`.
- **`@deprecated` in the LSP.** A use of a deprecated item is drawn
  struck through (the LSP `Deprecated` diagnostic tag). When `replacement:`
  is a plain identifier, a quick fix renames the use to it.
- **`kestrel-doc`.** Shows `@deprecated` (with its message) and the derived
  conformances on each type page.

## Part B: Built-in attributes

### B1. `@inline` (#111, adopted)

| Written | LLVM | Cranelift |
|---|---|---|
| `@inline` | `inlinehint` | ignored |
| `@inline(.Always)` | `alwaysinline` | ignored |
| `@inline(.Never)` | `noinline` | ignored |

- **Targets.** Free functions, methods, initializers, `deinit`, subscripts,
  and computed properties (the attribute applies to every accessor). It is
  allowed in a protocol extension. It is **not** allowed on a bodiless
  requirement (E905) or on an `@extern` (conflict, A7).
- **Lowering.** `FunctionDef::inline`, set in `function_sig.rs`. Every mono
  instance of a generic function gets it.
- **Thunks.** Witness thunks and closure thunks are not marked. The marked
  function is a direct callee of the thunk, so LLVM can still inline it into
  the thunk.
- **Backend mapping.** #111 also maps `-O0` to Cranelift and `-O1`/`-O2` to
  LLVM. That is a driver change (`src/main.rs:122`, where `--backend`
  defaults to Cranelift today), tracked in #111, and independent of this
  design.
- **`.Always` on a recursive function.** No diagnostic. LLVM keeps the call,
  as it does in Rust and Swift.

### B2. `@deprecated`

```kestrel
@deprecated
@deprecated("use 'parse(_:)' instead")
@deprecated("unsafe on empty input", since: "0.18", replacement: "parse")
```

| Parameter | Kind | Meaning |
|---|---|---|
| positional, optional | string | The message, shown in the warning |
| `since:` | string | The version of the **declaring package** that deprecated it. Shown in the warning, not checked against anything |
| `replacement:` | string | The name to use instead. Shown as "use 'X' instead", and used by the LSP quick fix when it is a plain identifier |

- **Targets.** Every named declaration: struct, enum, protocol, enum case,
  type alias, associated type, function, method, requirement, initializer,
  subscript, field, computed property, module value. On an `extend` it
  deprecates every member declared in that extension (not the conformance:
  see Edge cases).
- **What warns (E908, warning).** Every *use*, resolved after type inference:
  - a call or reference to a deprecated function, initializer, subscript,
    case, field or property;
  - a deprecated type, protocol or alias named anywhere a type is written,
    including signatures, conformance lists, `where` clauses, `some P` and
    `any P`;
  - a call through a generic bound or an existential to a deprecated
    requirement.
- **What does not warn:**
  - the declaration itself;
  - uses inside a declaration that is itself deprecated, or nested in one.
    This lets deprecated APIs call each other during a migration, as Swift and
    Rust do;
  - implementing a deprecated requirement in a conformance;
  - compiler-generated code: derived conformances, `@main` wrappers, thunks.
- **Where it runs.** A body analyzer walks the resolved references in HIR
  bodies. A declaration analyzer walks resolved signature and conformance
  types. Both use one query, `DeprecationOf(entity) -> Option<&Deprecation>`,
  which also checks the parent extension.
- **Silencing.** There is none in 0.18. A general `@allow(deprecated)` belongs
  to lint configuration, which is out of scope.

### B3. Existing attributes, through the registry

| Attribute | Targets | Arguments | Phase | Change from today |
|---|---|---|---|---|
| `@main` | free function | none | Semantic | None (E615–E618 stay) |
| `@extern` | free function without a body | `Cases[C]` (required, E612 when missing); `mangleName: Str` | Lowering | **`.C` is now checked.** Anything else is E903, instead of being compiled as C |
| `@builtin` | any declaration | `OpenCase(Builtin::from_attribute_name)` (required) | NameRes | An unknown builtin name becomes E903 instead of being ignored |
| `@fileconstant` | module value | `Str` (required) | Lowering | The path is unescaped once, by `AttrsOf` |
| `@platform` | any declaration, including `extend` | `Cases[darwin, linux]`, variadic, at least one | Build | Variadic *any-of*; not repeatable (A7) |
| `@dummy` | any | `Any` | — | Kept for parser tests |

**`@platform` case spelling.** `@platform` uses lowercase cases (`.linux`, 35
uses in `lang/`). `@builtin` and `@extern` use PascalCase, as enum cases do
everywhere else (`.Some`, `.Less`). **John decides:** keep `.linux` (no churn)
or rename to `.Linux` and `.Darwin`. If renamed, accept both for one release
with a warning on the old spelling. Recommendation: rename, because `.linux`
is the only lowercase case in the language.

## Part C: Derives

### C1. Explicit, not implicit

A struct gets `Equatable` (or any derived protocol) only when it says
`@derive(Equatable)`. Copyability is different: `Copyable` and `Cloneable`
stay inferred, as today.

The two kinds of protocol differ in what they promise:

- **`Copyable`/`Cloneable` describe memory.** They answer "what does `let b =
  a` do", which has to have an answer for every type. The fold computes it from
  the fields. Opting out is `not Copyable`.
- **`Equatable`, `Hashable`, `Comparable`, `Formattable` and `Defaultable` are
  API promises.** If they were implicit:
  - every public struct would expose equality and ordering over its private
    fields;
  - reordering enum cases would silently change `Comparable` results;
  - adding a field could silently drop a conformance that downstream code
    depends on;
  - overload resolution would see conformances nobody asked for.

  Swift and Rust both make these opt-in.

`docs/memory-model/cloneable.md` raises the question for `Cloneable`; this
design keeps it automatic. **John decides** (D-Q1), recommendation: explicit
`@derive` for API protocols, inference for the memory model.

**`@derive(Cloneable)` and `@derive(Copyable)`.** The roadmap lists
`Cloneable` in the derive list. That was written before the copy fold made it
automatic. Deriving it now would either do nothing (the type is already
`Cloneable` or `Copyable` by its fields) or contradict `not Copyable` (E423).
Recommendation: reject both with E951, worded to explain the inference:
"'Cloneable' is not derived: 'Document' is Cloneable automatically because a
field is; declare 'Cloneable' only to hand-write 'clone()'". **John decides**
(D-Q2): reject (recommended), or accept as a checked assertion that the fold
gives `Cloneable`.

### C2. `@derive`

```kestrel
@derive(Equatable, Hashable)
struct Point { var x: Int64; var y: Int64 }
```

- **Arguments** are protocol paths (`ParamKind::Path`, variadic, at least
  one). Name resolution resolves each path in the declaration's scope.
  Anything that is not a protocol is E950.
- **Targets:** structs, enums, and extensions of a struct or enum (§C3).
- **Not allowed** on protocols, type aliases, or an extension whose type is
  declared in another package. A derivation reads stored fields, and another
  package's fields are not part of its API. That case is E954.

**What it means.** `@derive(P)` on a type declaration is sugar for an
extension:

```kestrel
@derive(P) struct S[T, U] { ... }
// means
@derive(P) extend S[T, U]: P where <derived bounds> { }
```

- **Desugared at build time.** The AST builder spawns that extension entity
  next to the type, with a `DerivedFrom { attr_span }` marker. It is an
  ordinary extension: `ExtensionsFor`, `ConformingProtocols`
  (`kestrel-name-res/src/conformances.rs:29`), conditional-conformance
  checking and witness lowering (`kestrel-mir-lower/src/items/witness_lower.rs:52`)
  all see it with no changes.
- **Rebuilt with its type.** The extension comes from the same syntax node,
  so it is rebuilt exactly when the type is. No new incremental input.
- **No synthesized members.** The extension's body is empty. Its witnesses
  come from P's derivation (§C4), exactly as a requirement with a default
  is witnessed by a protocol extension today.

### C3. Conditional derives on generic types

**Derived bounds.** A type parameter `T` gets the bound `T: P` if it appears
anywhere in the type of a stored field or of an enum payload. Otherwise it gets
no bound.

| Declaration | Derived conformance |
|---|---|
| `@derive(Equatable) struct Box[T] { var value: T }` | `Box[T]: Equatable where T: Equatable` |
| `@derive(Hashable) struct Bag[T] { var items: Array[T] }` | `Bag[T]: Hashable where T: Hashable` |
| `@derive(Equatable) struct Tagged[Tag] { var id: Int64 }` (phantom) | `Tagged[Tag]: Equatable`, unconditional |
| `@derive(Equatable) enum Either[L, R] { case Left(L); case Right(R) }` | `where L: Equatable, R: Equatable` |

- **Why syntactic.** It can be computed in the builder from the field
  `AstType`s, before any types are resolved. That is what lets §C2 desugar to
  a real extension.
- **Field check.** `DerivedFieldConformance` then checks, with the
  resolved types, that every field conforms under those bounds. Any field
  that does not is E952, naming the field.
- **When the derived bound is too strong.** For example, a
  `Pointer[T]` field is `Equatable` for every `T`. In that case the user
  writes the extension themselves and derives on it:

  ```kestrel
  struct Handle[T] { var raw: Pointer[T] }
  @derive(Equatable) extend Handle[T] { }          // unconditional
  ```

  `@derive(P)` on an extension adds `P` to that extension's conformances and
  marks it derived. Its own `where` clause is used as written, and nothing is
  inferred.

**Relation to 0.20 "conditional conformance".** Derives need nothing from
0.20. The `where`-gated extension conformance they desugar to already
type-checks, monomorphizes and dispatches. The stdlib's `Optional`, `Result`
and `Array` conformances depend on it (see [Current state](#current-state)).
`docs/memory-model/generics.md` already documents conditional `Copyable`.
**John decides** (D-Q3): confirm that 0.20's conditional-conformance item is
about something else (overlap rules, or `Copyable` on user generics?). If
0.20 changes how a `where`-gated conformance is represented, derives follow
it, because they use that representation and do not copy it.

### C4. Defining what can be derived: `@derivation`

This is the general mechanism. **A protocol `P` is derivable iff its module
contains exactly one protocol extension of `P` marked `@derivation`.** That
extension implements `P`'s requirements generically over `Self`, using the
structural API `Derive` (§C5) to visit the fields of whatever type is
deriving.

```kestrel
// lang/std/core/protocols.ks
@derivation
extend Equatable {
    public func isEqual(to other: Self) -> Bool {
        // Same case (always true for a struct), then every field equal.
        Derive.all(self, other) { (_, a, b) in a.isEqual(to: b) } ?? false
    }
}
```

**How it is checked:**

- **Once, at the protocol.** The extension is type-checked like any protocol
  extension. `Self` is the abstract conformer and `Self: P` holds. A bug in a
  derivation is an error in the protocol's module, not at every `@derive`
  site.
- **How it witnesses.** Its members witness `P`'s requirements **only for
  derived conformances**, meaning extensions carrying `DerivedFrom` or
  `@derive`. For every other conformer they are invisible. They are not
  defaults, and they never appear in member lookup on a concrete type.
- **Witness order for a derived conformance:** the derivation's member first,
  then an ordinary protocol-extension default. A member the type wrote by hand
  is a conflict (E953): the user asked for a derived conformance and then
  wrote part of it.
- **Every requirement must be covered.** Each requirement must have a member
  in the derivation or an ordinary default. A gap is E955, reported at the
  `@derivation` extension.

**Constraints on derivations:**

| Rule | Why |
|---|---|
| In the same module as `P` | Coherence: one derivation per protocol, and it cannot be replaced from outside |
| At most one per protocol | Same |
| `Derive.*` is only allowed inside a `@derivation` extension | It needs a concrete `Self` at mono time, which only derived conformances guarantee (§C5) |
| May not be applied to a protocol with a `Self` associated-type requirement the derivation cannot bind | The derivation can only bind associated types to types computable from `Self` (as `extend P { type X = ... }` already does) |

**Why this design, not compiler-built derivers.** Rust's built-in derives
(before proc macros) and Swift's synthesized conformances are compiler code,
one per protocol. That would be faster to ship, but:

- every new derivable protocol would need a compiler change;
- users could never make their own protocols derivable;
- 2.0 metaprogramming would have to replace it rather than extend it.

A derivation is ordinary Kestrel. It is type-checked once and documented like
any extension, and it reads as the specification of what deriving means. The
only compiler-specific parts are the `Derive` forms. They are a deliberately
small subset of the 2.0 roadmap's "compile-time reflection (inspecting types,
fields...)": structural folds only. They do no name lookup, no code
generation and no type construction. **John decides** (D-Q4): this mechanism
(recommended), or compiler-built derivers for the five stdlib protocols with
`@derivation` deferred to 2.0. The `@derive` surface and semantics in
§C1–§C3 are the same either way.

### C5. The `Derive` structural API

`Derive` is a stdlib namespace (`std.core.Derive`, an uninhabited enum of
`@builtin` static members). Its members are only callable inside a
`@derivation` extension (E956 elsewhere).

**The model.** Every deriving type is seen as a list of cases, each with a
list of fields:

- a **struct** is one case, whose fields are its stored fields in declaration
  order;
- an **enum** has its cases in declaration order, and each case's fields are
  its payload.

Fields removed by `@platform` are not there. Computed properties and statics
are not fields.

```kestrel
/// Static facts about one field, built by the compiler as constants.
public struct FieldInfo {
    public let index: Int64     // position within its case, from 0
    public let label: String    // field name or payload label; "" if unlabeled
}
```

**Constant queries:**

| Member | Value |
|---|---|
| `Derive.typeName: String` | The type's name, without module or type arguments (`"Box"`) |
| `Derive.isEnum: Bool` | Whether `Self` is an enum |
| `Derive.caseIndex(of: Self) -> Int64` | The value's case, in declaration order. Always `0` for a struct |
| `Derive.caseName(of: Self) -> String` | The case name. `typeName` for a struct |
| `Derive.fieldCount(of: Self) -> Int64` | Number of fields in the value's case |

**Folds.** Each fold takes a block that is called once per field, in order.

| Form | Block | Result | Stops early |
|---|---|---|---|
| `Derive.each(x) { (info, v) in ... }` | `-> ()` | `()` | no |
| `Derive.all(x) { (info, v) in ... }` | `-> Bool` | `Bool` | on the first `false` |
| `Derive.first(x, unless: r) { (info, v) in ... }` | `-> R` | the first result `!= r`, else `r` (`R: Equatable`) | yes |
| `Derive.fold(x, from: a) { (info, acc, v) in ... }` | `-> A` | final `acc` | no |
| two-value forms `each(x, y)`, `all(x, y)`, `first(x, y, unless:)`, `fold(x, y, from:)` | `(info, a, b)` | `Optional` of the one-value result. **`.None` when `x` and `y` are in different cases** | as above |
| `Derive.build(case: k) { (info) in ... }` | `-> Field` | a new `Self` | — |

- **Why two-value forms return `Optional`.** For an enum, "are these in the
  same case?" must be answered before fields can be paired. Returning
  `Optional` makes the derivation decide what different cases mean, in the
  type: `?? false` for `Equatable`, compare `caseIndex` for `Comparable`. No
  derivation can forget it, and nothing traps. For a struct the result is
  always `.Some`, and the optimizer removes the wrap.
- **`build` constructs.**
  - It initializes each field from the field's **declared initializer** if
    it has one (`var retries: Int64 = 3`). Otherwise it calls the block.
  - `case:` picks the enum case. It is omitted for a struct, and must be a
    case index known at compile time: an integer literal or
    `Derive.caseMarked("default")` (§C6). A non-constant `case:` is E956.

**How a block is typed.** This is the one new type-checking rule. A fold
block is checked **once**, as if it were a nested generic function:

```kestrel
func block[Field](info: FieldInfo, v: Field) -> R
    where Field: P, Field: not Copyable, Field: not Static
```

- **`P` is the derivation's protocol.** So a block can call `P`'s members on a
  field and nothing else. That is also exactly the bound the field check
  (§C3, E952) enforces at each `@derive` site.
- **`not Copyable`, `not Static`.** Fields may be move-only or hold
  references, so the block cannot copy a field. It can only borrow `v`, by the
  ordinary parameter convention. This is why derives work unchanged on
  `not Copyable` types.
- **`Field` is nameable inside the block.** A `build` block returns `Field`
  (`Field()` for `Defaultable`).
- **Captures.** The block is a view-kind closure (`docs/design/closures.md`).
  It may read and mutate enclosing locals and parameters, such as the
  `hasher` being written into. It cannot escape.
- **Generic callees.** `Derive.*` members have no expressible Kestrel
  signature, so the solver special-cases calls to them by `Builtin` id. It
  types the block as above and checks the result type against the form.

**How it lowers:**

- **Generic MIR.** The block lowers to a view closure whose body is generic
  over one more type parameter, `Field`, after the enclosing ones. Each
  `Derive.*` call lowers to a `MirIntrinsic::Derive { form, block, operands }`.
- **Mono.** When a derivation member is instantiated at a concrete `Self`
  (derived conformances guarantee it always is), mono expands the intrinsic
  using `Self`'s MIR layout:
  - **Struct:** one call to `block[F_i]` per field `i`, passing a borrow of
    `self.field_i` and a constant `FieldInfo`. `all` and `first` branch out
    after each call.
  - **Enum:** a `switch` on the discriminant (two discriminants for the
    two-value forms). Each arm binds its payload by borrow and expands as for
    a struct. Mismatched-case arms produce `.None`.
  - `caseIndex` and the other queries lower to a discriminant read or a
    constant.
- **No new runtime machinery.** The expansion uses calls, switches and
  borrows, all of which exist. It never fails, because §C3 has already proved
  every `F_i: P`.
- **Incremental.** A derivation depends only on `P`'s module. A derived
  conformance depends on the type's field list, which is already a
  declaration input. The expansion happens in mono, which is whole-program
  today (see existentials §5, "Where tables live").

### C6. The stdlib derivations

**Equatable**, shown in §C4. Two values are equal iff they are in the same
case and every field is equal. Fields are compared in order, stopping at the
first difference.

**Hashable.** Feeds the case index (enums only), then each field in order.

```kestrel
@derivation
extend Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        if Derive.isEnum { Derive.caseIndex(of: self).hash(into: hasher) }
        Derive.each(self) { (_, v) in v.hash(into: hasher) }
    }
}
```

- **Why hash the case index.** It keeps `.A(1)` and `.B(1)` apart.
- **Why only for enums.** Structs skip it, so a one-field struct hashes the
  same as its field. That is harmless, and matches what a hand-written
  conformance would usually do.
- **Payload-less enums** hash the index only.

**Comparable.** Orders by case index (declaration order) first, then fields
lexicographically.

```kestrel
@derivation
extend Comparable {
    public func compare(other: Self) -> Ordering {
        Derive.first(self, other, unless: .Equal) { (_, a, b) in a.compare(other: b) }
            ?? Derive.caseIndex(of: self).compare(other: Derive.caseIndex(of: other))
    }
}
```

**Declaration order is the order.** Reordering cases is a breaking change for
a type that derives `Comparable`. The derive docs say so, and so do the E952
notes for enums.

**Formattable.** `Version(major: 1, minor: 2, patch: 0)`.
`Shape.Circle(radius: 1.0)` prints as `Circle(radius: 1.0)`. A payload-less
case prints its name. An unlabeled payload prints positionally:
`Some(3)`.

```kestrel
@derivation
extend Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.write(Derive.caseName(of: self))
        if Derive.fieldCount(of: self) == 0 { return }
        writer.write("(")
        var fieldOptions = options;
        fieldOptions.debug = true
        Derive.each(self) { (info, v) in
            if info.index > 0 { writer.write(", ") }
            if info.label != "" { writer.write(info.label); writer.write(": ") }
            v.format(into: writer, options: fieldOptions)
        }
        writer.write(")")
    }
}
```

- **Fields are formatted with `debug: true`**, so a `String` field prints
  quoted: `User(name: "Ada")`, not `User(name: Ada)`.
- **`Formattable` is also what string interpolation uses.** So a derived
  conformance gives a debug-style rendering for `"\(user)"` too.
- **John decides** (D-Q5): derive `Formattable` with this format
  (recommended), or wait for a separate debug-format protocol. Recommendation:
  derive it now. A type that wants a user-facing rendering writes it by hand,
  and that case is not what deriving is for.

**Defaultable.** Each field takes its declared initializer, or `Field()`.

```kestrel
@derivation
extend Defaultable {
    public init() {
        self = Derive.build(case: Derive.caseMarked("default")) { (_) in Field() }
    }
}
```

- **Field check.** A field with a declared initializer is exempt from the
  E952 check for `Defaultable`. `build` never calls the block for it, so it
  does not need to be `Defaultable` itself. In general, a field reached
  *only* through `build` and having an initializer is exempt.
- **Enums.** An enum derives `Defaultable` only with exactly one case marked
  `@default` (E957 otherwise).
  - That case's payload fields follow the same rule as struct fields.
  - `Derive.caseMarked("default")` is the compile-time index of that case.
    For a struct it is `0`.
  - `@default` is a built-in attribute: target `EnumCase`, no arguments.
- **`self =` in `init`.** If assigning `self` inside an initializer is not
  supported yet, `Derive.build` is accepted as the initializer's only
  statement and lowered to per-field initialization. That is a lowering
  detail, not a surface change.
- **John decides** (D-Q6): derive `Defaultable` for structs and `@default`
  enums (recommended), or structs only.

**Derivations not in 0.18:**

- `Cloneable` stays the MIR clone shim. A `Derive.map` form could express it
  later and retire the shim, but the shim's interaction with drop
  elaboration is not worth disturbing in this release.
- `Matchable`, `Iterator` and other protocols whose meaning is not
  structural are never derivable.

### C7. Rules at the `@derive` site

Checked by one `DeriveSite` analyzer, in this order:

1. **Resolve.** Every argument resolves to a protocol (E950). That protocol
   has a `@derivation` (E951; `Copyable`/`Cloneable` get the C1 wording).
2. **No duplicate conformance.** The type does not also declare `P` in its
   conformance list or in another extension (E953 with the "already declares"
   wording). Deriving `P` twice in one list is the generic duplicate-argument
   error.
3. **Parents.** Every protocol `P` refines must hold under the derived
   bounds: derived in the same `@derive`, declared, or inferred. `@derive(Hashable)`
   without `Equatable` is E958: "deriving 'Hashable' requires 'Point' to be
   'Equatable'; add 'Equatable' to '@derive'". Parents are **not** derived
   implicitly, so the list states exactly what the type promises, as in Rust.
4. **Fields.** Every field conforms to `P` under the derived bounds (E952),
   with the `build` exemption from §C6.
5. **No hand-written witnesses.** No member written on the type itself, in
   any of its extensions, witnesses a requirement of a derived `P` (E953).
6. **Sized.** The type has no unsized tail (`where T: not Sized` as its last
   field, from the existentials design §2). Otherwise it is E954. Fold blocks
   take `Field` by borrowed value, which an unsized field cannot provide.
   Lifting this needs `Field: not Sized` in the block, and that can follow
   when a real use appears.

### C8. Interactions

- **Copyable inference.** A derived conformance is an ordinary conformance. It
  never changes the copy classification, and the classification never changes
  what is derived. A `not Copyable` type derives everything in §C6, because
  every fold borrows.
- **Conditional `Copyable` containers.** `enum Box[T]: not Copyable` plus
  `extend Box[T]: Copyable where T: Copyable` is unaffected.
  `@derive(Equatable)` adds a second, independent `where`-gated extension.
- **Existentials.** `any P` for a derived `P` behaves as for a hand-written
  one. Derived `Equatable` and `Comparable` take `Self`, so they are not
  dispatchable (existentials §5), exactly as hand-written ones are.
- **`@platform`.** Fields and cases filtered out by `@platform` don't exist
  when deriving. Case indices are counted after filtering.
- **`@deprecated`.** Derived code never warns. A deprecated field is still
  compared, hashed and formatted.
- **Visibility.** `Derive` folds read private fields. That is the point of
  deriving. Only the derivation sees them, through borrowed `Field` values.
  The derivation cannot name them.
- **Indirect enums.** Not shipped. When they are, an `indirect` payload is
  visited through its box's `Indirection`, so the block still sees the
  payload type.
- **Recursive types.** `@derive(Equatable) enum List { case Nil; case Cons(Int64, Box[List]) }`
  requires `Box[List]: Equatable`. That holds through the box's own
  conditional conformance, which in turn needs `List: Equatable`. This is
  the derived conformance itself. The conformance solver must accept this
  cycle as it would for a hand-written conformance. M6 adds an execution test
  for it, since this design has not verified that it does today.

## Pipeline impact

| Stage | Change |
|---|---|
| **Syntax / parser** | `AttributeList?` on `ExtensionDeclaration` and `DeinitDeclaration`; allow an attribute head before `extend`/`deinit` (`items.rs:141`) |
| **AST builder** | `attrs/` module: registry, `AttrTarget`, `AttrValue`, argument parser (shared by `@platform` filtering), per-arg spans; `AttrsOf` query; spawn derived extension entities with `DerivedFrom` and derived bounds |
| **Name res** | Resolve `@derive` paths; `ConformingProtocols` sees derived extensions with no change; `EntityBuiltin` reads `AttrsOf`; `DerivationOf(P)` query (the unique `@derivation` extension) |
| **Type infer** | Witness resolution for derived conformances prefers the derivation's members; typing of `Derive.*` calls and fold blocks (the implicit `Field` generic); `DeprecationOf` is used by the use-site walk |
| **Analyzers** | `AttributeValidation` (replaces `unknown_attribute.rs`); `DeprecatedUse` (body and declaration halves); `DeriveSite` (§C7); `DerivationDecl` (§C4 rules, E955/E956); existing `@main`/`@extern`/`@builtin` analyzers read `AttrsOf` |
| **MIR lower** | `FunctionDef::inline`; fold blocks as view closures generic over `Field`; `MirIntrinsic::Derive`; consumers switch to typed accessors |
| **Mono** | Expand `MirIntrinsic::Derive` at concrete `Self` (§C5); copy `inline` to instances |
| **Codegen** | LLVM: `alwaysinline` / `inlinehint` / `noinline`. Cranelift: none |
| **Stdlib** | `Derive`, `FieldInfo`; `@derivation` extensions for `Equatable`, `Hashable`, `Comparable`, `Formattable`, `Defaultable`; `InlineMode`, `CallingConvention`, `Platform` documentation enums |
| **LSP / doc** | Attribute completion and hover from the registry; deprecated tag and quick fix; derived conformances on type pages |

## Milestones

Each milestone ships green and is useful on its own.

| # | Scope | Issue |
|---|---|---|
| M1 | Registry, typed values, `AttrsOf`, `AttributeValidation`; migrate the six existing attributes; per-arg spans. No behaviour change beyond the new errors | #32 |
| M2 | Attributes on `extend` and `deinit`; variadic `@platform` | #32 |
| M3 | `@inline` → MIR → LLVM | #111, #34 |
| M4 | `@deprecated` and both use-site walks; LSP deprecated tag | #34 |
| M5 | `@derive` desugaring, derived bounds, `DeriveSite` checks; compiler-side stub derivations for `Equatable`/`Hashable` to validate the desugaring before M6 | #33 |
| M6 | `@derivation`, `Derive` API, fold-block typing, mono expansion; stdlib derivations for `Equatable`, `Hashable`, `Comparable`; remove M5's stubs | #33 |
| M7 | `Formattable`, `Defaultable` + `@default` | #33 |

M6 is the largest and riskiest piece: the fold-block typing and the mono
expansion. If 0.18 runs short, M7 moves to 0.19. M6 has no smaller useful
form, which is the cost of D-Q4's recommendation.

## Error cases

**John decides** (D-Q7): allocation. E4xx has seven free codes left
(E405–E410, E414; `lib/kestrel-analyze/AGENTS.md`), which is not enough, and
E9xx is unused. Proposal: a new band **E900–E999, "Attributes and derives"**,
recorded in `lib/kestrel-analyze/AGENTS.md`. E900–E949 cover attributes and
E950–E999 cover derives.

| Code | Condition | Diagnostic |
|---|---|---|
| E461 (existing) | Unknown attribute | `unknown attribute '@inlne'; did you mean '@inline'?` |
| E900 | Attribute not allowed here | `'@inline' cannot be applied to a struct; it applies to functions, initializers, subscripts and computed properties` |
| E901 | Non-repeatable attribute repeated | `'@platform' appears twice; list every platform in one '@platform(...)'` (generic: `duplicate attribute '@inline'`) |
| E902 | Missing required argument (E612 for `@extern`) | `'@fileconstant' needs a path: '@fileconstant("data/table.bin")'` |
| E903 | Unknown case, or wrong value kind | `'.Sometimes' is not an inline mode; expected '.Always' or '.Never'` (variants: `expected a string literal`, `unknown builtin '.Foo'`, `unsupported calling convention '.Rust'; only '.C' is supported`) |
| E904 | Unknown, repeated or misplaced label | `'@deprecated' has no argument 'renamed'; did you mean 'replacement'?` (variants: `argument 'since' given twice`, `positional argument after a labeled one`) |
| E905 | Attribute needs a body | `'@inline' needs a function body; 'area' is a protocol requirement` |
| E906 | Conflicting attributes | `'@inline' cannot be combined with '@extern': an extern function has no body to inline` |
| E907 | Too many positional arguments | `'@inline' takes at most one argument` |
| E908 | Use of a deprecated item (**warning**) | `'legacyParse' is deprecated since 0.18: unsafe on empty input; use 'parse' instead` |
| E950 | `@derive` argument is not a protocol | `'Point' in '@derive' is not a protocol` (variant: `cannot find protocol 'Equtable'`) |
| E951 | Protocol is not derivable | `'Iterator' cannot be derived: it has no '@derivation'` (variant for `Copyable`/`Cloneable`, §C1) |
| E952 | Field does not conform | `cannot derive 'Hashable' for 'Point': field 'tag' has type 'Tag', which is not 'Hashable'` (enum: `payload 'radius' of case 'Circle'`; generic note: `derived bound: 'T: Hashable'`) |
| E953 | Hand-written witness or duplicate conformance | `'Point' derives 'Equatable' but also implements 'isEqual(to:)'; remove one` (variant: `'Point' already declares 'Equatable'`) |
| E954 | `@derive` on an unsupported declaration | `'@derive' on an extension needs a type declared in this package` (variants: protocol, type alias, unsized tail) |
| E955 | Derivation doesn't cover a requirement | `the derivation of 'Hashable' does not implement 'hash(into:)'` |
| E956 | `Derive` misuse | `'Derive.each' can only be used in a '@derivation' extension` (variants: `'case:' must be a constant case index`, `a second '@derivation' for 'Equatable'`, `'@derivation' must be in the module that declares 'Equatable'`) |
| E957 | Enum `Defaultable` without exactly one `@default` case | `cannot derive 'Defaultable' for 'Mode': mark exactly one case '@default'` |
| E958 | Missing parent conformance | `deriving 'Hashable' requires 'Point' to be 'Equatable'; add 'Equatable' to '@derive'` |

**Unknown attributes become an error.** **John decides** (D-Q8). E461 is a
warning today, which is how `@inlne` silently does nothing. With a complete
registry there is no legitimate unknown attribute until 2.0 custom
attributes. Recommendation: make it an error and keep the code.

## Edge cases

- **Empty struct / payload-less enum.**
  - Equatable: always true, or same case.
  - Hashable: nothing, or the index.
  - Comparable: by index.
  - Formattable: the name.
  - Defaultable: the struct, or the `@default` case.
- **Single-field struct.** It hashes like its field (§C6); this is intended.
- **Generic phantom parameter.** No bound (§C3).
- **Floating-point fields.** Derived `Equatable` uses `Float64`'s own
  `isEqual`, so `NaN != NaN` carries through. Derived `Hashable` on a
  float field is legal iff `Float64: Hashable`. The derive does not change
  that question.
- **`@derive` on an extension with its own conformance list.**
  `@derive(Hashable) extend Point: Formattable { ... }` derives `Hashable` and
  declares `Formattable` by hand. Each is checked as usual.
- **`@deprecated` on an extension that adds a conformance.** It deprecates the
  extension's members, not the conformance. A conformance has no use site to
  warn at; generic code using `T: P` never names `S`.
- **Same field type in many derived types.** The block instance
  `block[Int64]` is shared, through ordinary mono dedup.
- **Derivation calling a requirement on `self`.** It is legal, and it is the
  user's recursion to avoid, as in any extension.
- **`Derive.caseIndex` on a struct in a derivation that also serves enums.**
  It is always `0`, so derivations need no struct/enum split.
- **`@inline` on a computed property with both `get` and `set`.** Both
  accessors are marked.
- **`@platform` on a derived type's field.** It is filtered before the field
  check and expansion, so one source can derive on two platforms with
  different field sets.

## Open questions

### For John

| # | Question | Recommendation |
|---|---|---|
| D-Q1 | Implicit or explicit derives? | Explicit `@derive` for `Equatable`/`Hashable`/`Comparable`/`Formattable`/`Defaultable`; `Copyable`/`Cloneable` stay inferred (§C1) |
| D-Q2 | `@derive(Cloneable)` / `@derive(Copyable)` | Reject with an explanation (E951); the roadmap's listing predates the copy fold |
| D-Q3 | Does 0.20's conditional conformance change anything derives rely on? | No: derives desugar to the `where`-gated extensions the stdlib already uses (§C3). Please confirm what 0.20 adds |
| D-Q4 | How derivable protocols are defined | `@derivation` extensions in Kestrel with the `Derive` fold API (§C4–C5). Fallback: compiler-built derivers, same `@derive` surface |
| D-Q5 | Derive `Formattable`? Format? | Yes: `Name(label: value, ...)`, fields with `debug: true` |
| D-Q6 | Derive `Defaultable`? | Yes: structs (declared initializers win) and enums with one `@default` case |
| D-Q7 | Diagnostic band | New E900–E999 band for attributes and derives |
| D-Q8 | Unknown attribute severity | Error |
| D-Q9 | `@platform` case spelling | Rename to `.Linux`/`.Darwin`, accepting the old spelling with a warning for one release |
| D-Q10 | `@deprecated` labels | `since:` and `replacement:` (Swift says `renamed:`; Rust has only `since`/`note`). `replacement:` reads as "what to use", which also covers non-rename migrations |

### Resolved

| # | Question | Resolution |
|---|---|---|
| 1 | Where attributes are declared | One Rust registry in `kestrel-ast-builder/src/attrs/`, shaped like a future Kestrel `attribute` declaration (§A2) |
| 2 | How consumers read attributes | Typed `ParsedAttr` values from the `AttrsOf` query; raw `Attributes` is the only stored form (§A5) |
| 3 | Validation | One `AttributeValidation` analyzer reports everything `AttrsOf` found; semantic checks keep their codes (§A5) |
| 4 | Enum-valued arguments | Matched by case name against the registry's list, as #111 decided; doc-only stdlib enums (§A4) |
| 5 | Propagation to MIR | Typed MIR fields set at lowering (`FunctionDef::inline`), no attribute list past the ECS (§A6) |
| 6 | Attributes on `extend` and `deinit` | Added in 0.18; parameters, statements, expressions later (§A1) |
| 7 | `@platform` repetition | Not repeatable; variadic any-of (§A7) |
| 8 | `@inline` | #111 as written; thunks unmarked; no-op in Cranelift (§B1) |
| 9 | `@deprecated` arguments | Optional positional message, `since:`, `replacement:` (§B2) |
| 10 | Where deprecation warns | Every resolved use outside a deprecated context; never in generated code; implementing a deprecated requirement doesn't warn (§B2) |
| 11 | What `@derive` is | Sugar for a derived `extend S: P where ...`, spawned by the builder; no synthesized members (§C2) |
| 12 | Conditional derives | Bounds on type parameters that appear in field types; an explicit `@derive` extension for anything else (§C3) |
| 13 | Enum case order for `Comparable` | Declaration order, after `@platform` filtering (§C6) |
| 14 | Hashing enums | Case index, then payload fields in order; structs skip the index (§C6) |
| 15 | Parent protocols | Must hold; never derived implicitly (§C7) |
| 16 | Hand-written member plus `@derive` | Error E953 (§C7) |
| 17 | Derives on `not Copyable` types | Supported; folds only borrow (§C5, §C8) |
| 18 | Unsized-tail types | Cannot derive in 0.18 (§C7) |
