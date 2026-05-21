# Protocol Witnesses

How protocol conformances are represented, dispatched, and resolved
in the MIR pipeline.

## Two kinds of type metadata

Following Swift SIL's separation, Kestrel distinguishes:

- **TypeInfo** (value-level): how a type exists in memory — copy behavior,
  drop behavior, layout. Lives on StructDef/EnumDef. See `types.md`.
- **WitnessDef** (protocol-level): what methods a type provides for a
  protocol conformance. Separate from TypeInfo.

TypeInfo answers "how do I copy/drop/lay out this value?" WitnessDef
answers "what function implements Protocol.method for this type?"

The old design conflated these by having the deinit/clone machinery fish
through protocol witnesses to find the right method entity. The new design
keeps them orthogonal.

## WitnessDef

```rust
struct WitnessDef {
    protocol: Entity,
    implementing_type: TyId,              // pattern — may contain TypeParam wildcards
    constraints: Vec<WhereConstraint>,    // conditional conformance requirements
    type_bindings: Vec<(Entity, TyId)>,   // associated type entity → concrete type
    methods: Vec<WitnessMethodBinding>,
}

struct WitnessMethodBinding {
    key: WitnessMethodKey,
    func: Entity,               // the concrete function that implements this method
    type_args: Vec<TyId>,       // type args for the concrete function
}

struct WitnessMethodKey {
    name: String,
    // disambiguates overloads within the same protocol
}
```

One WitnessDef per conformance *declaration*, not per concrete instantiation.
The `implementing_type` is a *pattern* — it may contain TypeParam wildcards.
For `extend Array: Equatable where Element: Equatable`, there's one
WitnessDef with `implementing_type: Named { entity: Array, type_args: [TypeParam(Element)] }`.

The monomorphizer matches concrete types (e.g. `Array[Int64]`) against
this pattern, binding `Element → Int64`, to resolve witness calls.
There is NOT a separate WitnessDef per concrete instantiation — patterns
are shared across all matching types.

## Witness dispatch in generic MIR

Generic code calls protocol methods via `Callee::Witness`:

```rust
Callee::Witness {
    protocol: Entity,           // e.g. Equatable
    method: WitnessMethodKey,   // e.g. "equals"
    self_type: TyId,            // e.g. TypeParam(T)
    method_type_args: Vec<TyId>,
}
```

This says "call Equatable.equals on whatever T turns out to be." The
concrete function is not known until monomorphization resolves T.

## Associated type resolution

Protocols can declare associated types:

```
protocol Container {
    type Element
    func get(at index: Int) -> Element
}
```

In generic MIR, `Container.Element` appears as:

```rust
MirTy::AssociatedProjection {
    base: TyId,             // the conforming type (e.g. TypeParam(T))
    protocol: Entity,       // Container entity
    assoc_type: Entity,     // Element entity (the associated type declaration)
}
```

Resolution happens via witness table lookup: find the WitnessDef for
`(T, Container)`, look up `type_bindings` for the Element entity, get
the concrete type. Both `AssociatedProjection` and `WitnessDef.type_bindings`
use the associated type's Entity as the key — no string lookups.

During monomorphization, all AssociatedProjection types are resolved to
concrete types. The MonoModule contains no AssociatedProjection.

## Witness pattern matching

Finding the right WitnessDef for a concrete type requires structural
pattern matching. A witness is declared for a pattern like
`Array[T]: Equatable` — the `T` in the pattern is a wildcard that binds.

```rust
fn find_witness(
    module: &MirModule,
    protocol: Entity,
    self_type: TyId,
) -> Option<(&WitnessDef, HashMap<Entity, TyId>)>
```

Returns the matching WitnessDef plus the bindings (e.g. `T → Int64` if
self_type was `Array[Int64]`). The bindings are used to substitute into
the witness's method type_args and associated type resolutions.

### Pattern matching rules

1. `Named { entity: E, type_args: [A, B] }` matches a witness with
   `implementing_type: Named { entity: E, type_args: [TypeParam(X), TypeParam(Y)] }`
   if E matches, binding X→A, Y→B.
2. Primitive types match exactly (no wildcards).
3. `TypeParam` in the pattern is a wildcard — binds to whatever the
   concrete type provides.
4. **Conditional conformances:** After pattern matching succeeds, check
   `constraints` against the bound type args. For `Array[T]: Equatable
   where T: Equatable`, matching `Array[NonEquatable]` succeeds the
   pattern match but fails the constraint check — the witness is rejected.
5. **Protocol inheritance:** if no direct witness exists for protocol P,
   check witnesses for descendant protocols that inherit from P. A
   witness for `Comparable` (which inherits `Equatable`) satisfies a
   lookup for `Equatable` — the more specific conformance implies the
   parent protocol's methods are available.

## Monomorphization resolves all witnesses

The monomorphizer:

1. Discovers all function instantiations via BFS from entry points
2. For each `Callee::Witness` in a body, resolves it to a concrete
   function entity via witness pattern matching
3. Rewrites the callee to `MonoCallee::Direct(MonoFuncId)`

After monomorphization:
- No `Callee::Witness` exists in any body
- No witness lookup happens at codegen time
- Every call is direct (known target) or indirect (Thin/Thick pointer)

## Drop shims and witnesses

Drop shims (`__drop$T`) are synthesized per-type, not per-protocol. They
read TypeInfo.drop to determine what cleanup is needed:

- Call the user-defined deinit (if any)
- Recursively call `__drop$F` for each droppable field

This is NOT witness dispatch — it's direct structural recursion over the
type's fields. The distinction matters: TypeInfo.drop is computed once
during lowering, not looked up through protocol witness tables.

Clone calls DO use witness dispatch: `Callee::Witness { protocol: Cloneable, method: "clone" }`.
Clone elaboration inserts these witness calls in generic MIR. Monomorphization
resolves them to direct calls in MonoModule.
