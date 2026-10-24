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
   compiler-built conformance. In 0.18 the compiler decides what can be
   derived: `Equatable`, `Hashable`, `Comparable`, `Formattable` and
   `Defaultable`, each with a built-in deriver. Letting a protocol's own
   module define its derive in code waits for 2.0 metaprogramming. Everything
   about `@derive` except where the witnesses come from is designed to stay
   the same through that change (§C6).

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
   `@derive` (§C2), `@platform` and `@deprecated` all
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
    Default,                       // `@default` on an enum case (§C5)
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
| `NameRes` | `@builtin`, `@derive`, `@default` | `kestrel-name-res` (builtin map, derive targets, derived extensions) |
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
  declared in another package. A deriver reads stored fields, and another
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
  are the deriver's MIR shims (§C4), registered the way the clone shim's
  are today.

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

### C4. Built-in derivers

**What can be derived is decided by the compiler in 0.18.** Five protocols
are derivable, each with a deriver written in Rust. Code-defined derivations
come with 2.0 metaprogramming (§C6). The `@derive` surface, its desugaring,
the derived bounds, the site checks and the diagnostics are all designed so
that the later move changes only where the witnesses come from.

```rust
/// The derivable protocols. One variant per deriver; adding a derivable
/// protocol before 2.0 is a new variant plus its generator.
pub enum DeriveKind { Equatable, Hashable, Comparable, Formattable, Defaultable }
```

**Which protocol is which.** Each kind is tied to its protocol by a
`Builtin`, never by name. `Formattable` already has `@builtin(.FormattableProtocol)`
(`format.ks:376`). M5 adds `EquatableProtocol`, `HashableProtocol`,
`ComparableProtocol` and `DefaultableProtocol` to `kestrel-hir/src/builtin.rs`
and marks the four stdlib protocols. A protocol is derivable iff it is the
target of one of those builtins. A user protocol named `Equatable` is not.

**How a deriver produces witnesses.** The generated witnesses are MIR shims,
the way `synthesize_clone_shims` (`kestrel-mir/src/passes/clone_shim.rs:33`)
already builds and registers `__clone$T` for inferred `Cloneable` types:

- **One pass, `synthesize_derive_shims`, next to the clone pass.** For each
  derived extension, the pass generates one MIR function per requirement of
  `P`, named like `__derive$Equatable$Point`, from the type's MIR field list
  or variant list. It registers the shim as the conformance's witness.
- **Generic types.** The shim is generic over the type's parameters, and
  `where` holds `T: P` from the derived extension. A call on a field of type
  `T` is an ordinary witness call that mono resolves per instantiation, as it
  does for any generic body. `Box[String]` and `Box[Int64]` get their own
  instances, and `Box[NotEquatable]` never gets one.
- **Signatures come from the protocol.** The shim's signature is the
  requirement's signature with `Self` substituted, read from the requirement
  entity. The deriver writes only the body, so the requirement's signature
  is still declared once, in the stdlib.
- **Type inference needs no bodies.** A call such as `a.isEqual(to: b)` on a
  `Point` resolves through the conformance to the protocol requirement, as
  `x.clone()` does today on a type whose `Cloneable` is inferred. Type
  inference only needs the conformance and the requirement's signature.
  The conformance-completeness analyzer skips extensions marked `DerivedFrom`
  or `@derive`, because their deriver supplies every requirement.
- **Nothing can fail in MIR.** The `DeriveSite` analyzer (§C7) has already
  proved every field conforms under the derived bounds. A shim can only call
  witnesses that exist, so there is no MIR-level diagnostic to design.
- **Fields are borrowed.** Shims read fields through borrows and never copy
  one, so every deriver works on `not Copyable` types.

**Why MIR shims and not generated source.** Generating Kestrel syntax (or
HIR) for each derived type and type-checking it would need member entities
spawned after name resolution, because the builder can't know which protocol
a path names. The clone shim already solves "a compiler-written witness for
every type that has a conformance", so derivers reuse that path. It is also
the cheapest piece to delete: in 2.0 the pass goes, and nothing upstream of
MIR changes (§C6).

### C5. What each deriver generates

The semantics below are what the shims implement. They are written as Kestrel
for readability. A struct is treated as an enum with one case.

**Equatable.** Same case, then every field equal, in declaration order,
stopping at the first difference.

```kestrel
// @derive(Equatable) enum Shape { case Circle(radius: Float64); case Rect(width: Float64, height: Float64) }
func isEqual(to other: Shape) -> Bool {
    match self {
        .Circle(a0) => match other { .Circle(b0) => a0.isEqual(to: b0), _ => false },
        .Rect(a0, a1) => match other {
            .Rect(b0, b1) => a0.isEqual(to: b0) and a1.isEqual(to: b1),
            _ => false,
        },
    }
}
```

**Hashable.** For an enum, the case index (as `Int64`) first, then each field
in order. A struct hashes only its fields.

- **Why hash the case index.** It keeps `.A(1)` and `.B(1)` apart.
- **Why only for enums.** A one-field struct then hashes the same as its
  field. That is harmless, and matches what a hand-written conformance would
  usually do.
- **Payload-less enums** hash the index only.

**Comparable.** Case index (declaration order) first, then fields
lexicographically: the first field whose `compare` isn't `.Equal` decides.

- **Declaration order is the order.** Reordering cases is a breaking change
  for a type that derives `Comparable`. The derive docs say so, and so do the
  E952 notes for enums.

**Formattable.** `Version(major: 1, minor: 2, patch: 0)`. An enum value prints
its case: `Circle(radius: 1.0)`. A payload-less case prints its name. An
unlabeled payload prints positionally: `Some(3)`.

- **Fields are formatted with `debug: true`.** So a `String` field prints
  quoted: `User(name: "Ada")`.
- **Width and padding** from the caller's `FormatOptions` apply to the whole
  value, not to each field.
- **Interpolation uses it too.** `Formattable` is what string interpolation
  uses, so a derived conformance gives a debug-style rendering for
  `"\(user)"`.
- **John decides** (D-Q5): derive `Formattable` with this format
  (recommended), or wait for a separate debug-format protocol. A type that
  wants a user-facing rendering writes it by hand; that isn't what deriving
  is for.

**Defaultable.** Each field takes its declared initializer
(`var retries: Int64 = 3`), or else `Field()`.

- **Field check.** A field with a declared initializer is exempt from the E952
  check for `Defaultable`.
- **Enums.** An enum derives `Defaultable` only when exactly one case is
  marked `@default` (E957 otherwise). That case's payload fields follow the
  struct rule.
- **`@default`** is a built-in attribute: target `EnumCase`, no arguments.
- **John decides** (D-Q6): derive `Defaultable` for structs and `@default`
  enums (recommended), or structs only.

**Not derivable in 0.18:**

- `Cloneable` stays inferred, through the existing clone shim (§C1).
- `Matchable`, `Iterator` and other protocols whose meaning is not
  structural are never derivable.

### C6. The path to code-defined derives (2.0)

The 2.0 roadmap's metaprogramming ("procedural macros (extending `@derive`
from 0.18), compile-time reflection") is where a protocol's own module defines
how it is derived. 0.18 keeps that path open by fixing everything except the
witness source.

**Kept unchanged when derives move into code:**

- `@derive(P)` syntax, targets, and its desugaring to a derived
  `extend S: P where ...` (§C2)
- derived bounds, and `@derive` on a hand-written extension (§C3)
- the `DeriveSite` checks and E950–E958 (§C7); only E951's wording changes,
  from "is not one of the derivable protocols" to "has no derivation"
- the semantics of the five stdlib derives (§C5), which their code
  replacements must reproduce; the 0.18 execution tests are the specification

**Changed in 2.0:**

- `DeriveKind`, the `*Protocol` derive builtins and `synthesize_derive_shims`
  are deleted.
- Each protocol's module supplies its derivation in Kestrel. One shape
  considered for this design is a protocol extension marked `@derivation`,
  written against a small structural fold API and checked once. For example:

  ```kestrel
  @derivation
  extend Equatable {
      public func isEqual(to other: Self) -> Bool {
          Derive.all(self, other) { (_, a, b) in a.isEqual(to: b) } ?? false
      }
  }
  ```

  Whether that or a general macro is the 2.0 mechanism belongs to the 2.0
  design, not this one.

**Not done in 0.18, to keep the path clean:**

- no user-definable derives of any kind, even for user protocols;
- no stdlib-side annotation beyond the `Builtin` markers, so 2.0 has no
  half-finished mechanism to remove.

### C7. Rules at the `@derive` site

Checked by one `DeriveSite` analyzer, in this order:

1. **Resolve.** Every argument resolves to a protocol (E950). That protocol
   is one of the derivable protocols in §C4 (E951; `Copyable`/`Cloneable`
   get the C1 wording).
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
   with the `Defaultable` exemption for fields with initializers (§C5).
5. **No hand-written witnesses.** No member written on the type itself, in
   any of its extensions, witnesses a requirement of a derived `P` (E953).
6. **Sized.** The type has no unsized tail (`where T: not Sized` as its last
   field, from the existentials design §2). Otherwise it is E954. A shim
   for an unsized tail would need the field's size and witnesses from the
   runtime table. That can follow when a real use appears.

### C8. Interactions

- **Copyable inference.** A derived conformance is an ordinary conformance. It
  never changes the copy classification, and the classification never changes
  what is derived. A `not Copyable` type derives everything in §C5, because
  the shims only borrow fields.
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
- **Visibility.** Derived shims read private fields. That is the point of
  deriving, and only compiler-generated code does it.
- **Indirect enums.** Not shipped. When they are, an `indirect` payload is
  visited through its box's `Indirection`, so the deriver still sees the
  payload type.
- **Recursive types.** `@derive(Equatable) enum List { case Nil; case Cons(Int64, Box[List]) }`
  requires `Box[List]: Equatable`. That holds through the box's own
  conditional conformance, which in turn needs `List: Equatable`. This is
  the derived conformance itself. The conformance solver must accept this
  cycle as it would for a hand-written conformance. M5 adds an execution test
  for it, since this design has not verified that it does today.

## Pipeline impact

| Stage | Change |
|---|---|
| **Syntax / parser** | `AttributeList?` on `ExtensionDeclaration` and `DeinitDeclaration`; allow an attribute head before `extend`/`deinit` (`items.rs:141`) |
| **AST builder** | `attrs/` module: registry, `AttrTarget`, `AttrValue`, argument parser (shared by `@platform` filtering), per-arg spans; `AttrsOf` query; spawn derived extension entities with `DerivedFrom` and derived bounds |
| **Name res** | Resolve `@derive` paths; `ConformingProtocols` sees derived extensions with no change; `EntityBuiltin` reads `AttrsOf`; `DeriveKindOf(P)` from the derive builtins |
| **Type infer** | None for derives (calls resolve through the conformance); `DeprecationOf` is used by the use-site walk |
| **Analyzers** | `AttributeValidation` (replaces `unknown_attribute.rs`); `DeprecatedUse` (body and declaration halves); `DeriveSite` (§C7); conformance completeness skips derived extensions; existing `@main`/`@extern`/`@builtin` analyzers read `AttrsOf` |
| **MIR lower** | `FunctionDef::inline`; consumers switch to typed accessors |
| **MIR passes / mono** | `synthesize_derive_shims` next to `synthesize_clone_shims` (§C4); copy `inline` to mono instances |
| **Codegen** | LLVM: `alwaysinline` / `inlinehint` / `noinline`. Cranelift: none |
| **Stdlib** | `@builtin(.EquatableProtocol)` etc. on the four protocols that lack one; `InlineMode`, `CallingConvention`, `Platform` documentation enums |
| **LSP / doc** | Attribute completion and hover from the registry; deprecated tag and quick fix; derived conformances on type pages |

## Milestones

Each milestone ships green and is useful on its own.

| # | Scope | Issue |
|---|---|---|
| M1 | Registry, typed values, `AttrsOf`, `AttributeValidation`; migrate the six existing attributes; per-arg spans. No behaviour change beyond the new errors | #32 |
| M2 | Attributes on `extend` and `deinit`; variadic `@platform` | #32 |
| M3 | `@inline` → MIR → LLVM | #111, #34 |
| M4 | `@deprecated` and both use-site walks; LSP deprecated tag | #34 |
| M5 | `@derive` desugaring, derived bounds, `DeriveSite` checks, derive builtins, `synthesize_derive_shims` for `Equatable`, `Hashable`, `Comparable` | #33 |
| M6 | `Formattable`, `Defaultable` + `@default` | #33 |

M5 is the largest piece. If 0.18 runs short, M6 moves to 0.19.

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
| E951 | Protocol is not derivable | `'Iterator' cannot be derived; the derivable protocols are Equatable, Hashable, Comparable, Formattable and Defaultable` (variant for `Copyable`/`Cloneable`, §C1) |
| E952 | Field does not conform | `cannot derive 'Hashable' for 'Point': field 'tag' has type 'Tag', which is not 'Hashable'` (enum: `payload 'radius' of case 'Circle'`; generic note: `derived bound: 'T: Hashable'`) |
| E953 | Hand-written witness or duplicate conformance | `'Point' derives 'Equatable' but also implements 'isEqual(to:)'; remove one` (variant: `'Point' already declares 'Equatable'`) |
| E954 | `@derive` on an unsupported declaration | `'@derive' on an extension needs a type declared in this package` (variants: protocol, type alias, unsized tail) |
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
- **Single-field struct.** It hashes like its field (§C5); this is intended.
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
- **Same field type in many derived types.** Each derived type gets its own
  shim, and the field witnesses they call are shared through ordinary mono
  dedup.
- **`@inline` on a computed property with both `get` and `set`.** Both
  accessors are marked.
- **`@platform` on a derived type's field.** It is filtered before the field
  check and shim generation, so one source can derive on two platforms with
  different field sets.

## Open questions

### For John

| # | Question | Recommendation |
|---|---|---|
| D-Q1 | Implicit or explicit derives? | Explicit `@derive` for `Equatable`/`Hashable`/`Comparable`/`Formattable`/`Defaultable`; `Copyable`/`Cloneable` stay inferred (§C1) |
| D-Q2 | `@derive(Cloneable)` / `@derive(Copyable)` | Reject with an explanation (E951); the roadmap's listing predates the copy fold |
| D-Q3 | Does 0.20's conditional conformance change anything derives rely on? | No: derives desugar to the `where`-gated extensions the stdlib already uses (§C3). Please confirm what 0.20 adds |
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
| 13 | Enum case order for `Comparable` | Declaration order, after `@platform` filtering (§C5) |
| 14 | Hashing enums | Case index, then payload fields in order; structs skip the index (§C5) |
| 15 | Parent protocols | Must hold; never derived implicitly (§C7) |
| 16 | Hand-written member plus `@derive` | Error E953 (§C7) |
| 17 | Derives on `not Copyable` types | Supported; shims only borrow (§C4, §C8) |
| 18 | Unsized-tail types | Cannot derive in 0.18 (§C7) |
| 19 | How derivable protocols are defined (was D-Q4) | Built-in derivers in the compiler for 0.18, chosen by John; code-defined derivations wait for 2.0 metaprogramming, with `@derive` designed to stay the same (§C4, §C6) |
