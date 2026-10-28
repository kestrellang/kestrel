# Root-cause analysis: generics / protocols bugs

Base: `pwd`=/home/user/kestrel, commit 3fea2041 ("fix(ci): make nightly CI green…"), prebuilt `target/release/kestrel`.
No source changes were made. All repros were re-run on this binary; the observed behavior matched REPORT.md.

## Group A: witness method binding ignores the return type (C5, C6, C25)

**Root cause.** `bind_witness_methods` → `find_impl_among` → `matches_candidate` (lib/kestrel-mir-lower/src/items/witness_lower.rs:249, :670, :730) picks the impl by name, labels and *param* types only. It ignores the return type. `S: Conv[Int64], Conv[String]` produces two WitnessDefs: `S:Conv[Int64]` and `S:Conv[String]`. `find_impl_among` returns the first `conv()` for both, which is `-> Int64`. Mono's `find_witness_with_method` (lib/kestrel-mir/src/mono/witness.rs:224) does select the `Conv[String]` witness correctly through `witness_proto_args_match`, but that witness's method binding already points at `conv -> Int64`.
- C5: `cs` mono calls `S.conv -> Int64`, and the result is used as a String, which segfaults (`dump mir -s mono -f Test2_cs` shows `call @mono(S.conv…RInt64)` typed `@owned String`).
- C6: the direct `S().conv()` with `let s: String` is lowered as `call @witness Conv.conv for S[String,String]` (raw MIR). It goes through the same misbound witness.
- C25: the same misbinding for `Conv[Bool]`. Cranelift's verifier catches the i64-vs-i8 return.

**Why it is missed.** The front-end analyzer (`conformance_completeness.rs:296-330`) already disambiguates same-shape overloads by return type (`method_return_type_matches`). The MIR binder never received that logic. `expected_param_types_for` substitutes only the parameter types.

**Confidence:** high.
**Fix shape:** have `matches_candidate` also compare the candidate's resolved return type against the requirement's return type substituted through `proto_subst`, or reuse the analyzer's chosen impl per (type, protocol-instantiation).

## Group B: conformance checking accepts mismatched signatures (C11, C12)

**C11 root cause.** `signatures_match` (lib/kestrel-analyze/src/compilation/conformance_completeness.rs:1428) compares only arity and labels, so `f(x: String)` satisfies `f(x: Int64)`. The MIR binder then also accepts it through `find_impl_among`'s label-only second pass (witness_lower.rs:678-683). Mono passes an Int64 where a String is expected, which segfaults.
**Fix shape:** compare each parameter type through the same conformance env used for return types (`type_compare_env_for_conformance`) and emit a "wrong parameter type" diagnostic. Remove or guard the label-only fallback in `find_impl_among`.

**C12 root cause.** `receivers_match` (conformance_completeness.rs:1445) checks only static vs instance. It does not check the receiver mode (`mutating` vs borrow). MIR then calls the mutating impl with `@borrow` (`dump mir -s mono -f Test1_g`: `call S.f(@borrow %v1)`), and it writes through the caller's `let` storage.
**Fix shape:** also require `p.receiver` mode == `i.receiver` mode (mutating impl may not witness a non-mutating requirement) in `receivers_match`.

**Confidence:** high for both.

## A2: protocol-extension (non-requirement) method dispatches to the concrete type's shadow

**Root cause.** A call to the extension-only method `x.g()` on `T: P` is lowered as `call @witness P.g for T` (raw MIR of `Test.run`). Witness generation treats every `ProtocolMembers` entry as a requirement, including entries with `member.extension.is_some()`. `bind_witness_methods` (witness_lower.rs:183-249) then resolves `g` through `TypeMembersByName`, where `S.g` (Direct) wins over the `ProtocolExtension` default.
**Locations:** witness_lower.rs:183 (method_entries built from all ProtocolMembers) and :280-320 (TypeMembersByName lookup). The emission is in body/call/mod.rs:544.
**Why it is missed.** Requirement and extension members share a single path. The analyzer skips `member.extension.is_some()` entries, but MIR does not.
**Confidence:** high.
**Fix shape:** for non-requirement extension members, emit a static `Callee` to the extension function with `self_type = T`, or bind the witness entry to `member.entity` directly instead of doing a type-member lookup.

## Group C: abstract protocol `Self` handling (M10, M11, H1)

**M11 root cause.** `WorldResolver::conforms_to`'s `TyKind::SelfType{entity: P}` arm (lib/kestrel-type-infer/src/resolve.rs:771) checks `ConformingProtocols(P).contains(P)`. `ConformingProtocols` (kestrel-name-res/src/conformances.rs:37) returns P's conformed and parent protocols, never P itself, so `Self !: P`.
**Fix shape:** in the SelfType arm, return true when `*entity == protocol` (or check the protocol closure including itself). **Confidence:** high.

**M10 root cause.** Member lookup on `SelfType{B}` falls into the generic nominal path of `resolve_member` (resolve.rs:~485-512). That path queries `TypeMembersByName(type_entity = B)`, which returns B's direct children, B's extensions, and the *extension defaults* of B's parent protocols. It does not return the parent protocol A's *requirements*. So `self.a()` is not found. The same lookup through a `T: B` param works (`resolve_param_member` → bound protocol closure).
**Fix shape:** route `TyKind::SelfType` (and `Protocol`) receivers through the param-style bound lookup (`ProtocolMembersByName` over the protocol closure). **Confidence:** medium-high.

**H1 root cause (CI bootstrap ICE).** `for (i, s) in a.iter().enumerate()` desugars to `Iterable.iter()` on an `EnumerateIterator`. That reaches the blanket default `extend Iterator: Iterable { func iter() -> Self { self } }` (lang/std/iter/iterator.ks:135). In that body, the abstract `Self` is classified as Copyable:
- type-infer: `copy_semantics_of` SelfType → `NominalCopySemantics(Iterator)` (resolve.rs:2524-2530)
- `solver_copy_class` SelfType → `instance_semantics(protocol, [])` (solver/conformance.rs:735)
- semantics `HirCopyLayer::member_semantics` SelfType → `base_semantics(protocol)` (kestrel-semantics/src/lib.rs, `HirTy::SelfType` arm)

A protocol entity is Copyable by default, so the borrowed-`self` return is accepted, and MIR lowers it as `copy_value %v0` (`dump mir -s verify`: `std.iter.Iterator.iter: %v1 = copy_value %v0`). `EnumerateIterator[I]` (adapters.ks:254, `I: not Copyable`) is not Copyable and not Cloneable. Mono's expand (lib/kestrel-mir/src/mono/expand.rs:1206-1225) finds a `copy_value` on a not-Copyable instantiation and, since 662be3af, reports it as a post-mono ICE. Before that commit it silently aliased the value, which is a latent double release. `ArrayIterator` does not trip the check because it is `Cloneable`, so its copy becomes a clone.

This is not specific to String: `[Int64]` with `.iter().enumerate()` gives the same ICE. Minimal repro:
```
module Test
@main
func main() -> lang.i64 { let a: [String] = ["x", "y"]; var n = 0; for (i, s) in a.iter().enumerate() { n = n + i; } if n != 1 { return 1 } 0 }
```
**Why it is missed.** Protocol `Self` falls back to the protocol entity's nominal copy semantics, as if it were a concrete type. It is not treated like a type param whose requirement depends on the protocol's declared bounds. Non-Copyable types are allowed to conform to `Iterator`, so "Self is Copyable" is unsound.
**Confidence:** high on the mechanism.
**Fix shape:** classify abstract `SelfType` as MayBeNonCopyable (unless the protocol refines Copyable) in all three classifiers. Make the stdlib blanket `iter` `consuming func iter() -> Self { self }`, and ensure the for-in desugar consumes the iterable.

## H3: generic struct with `escaping` closure field over an associated type

**Root cause.** `synthesize_clone_shims` skips every struct with `has_unresolvable_fields_struct` (lib/kestrel-mir/src/passes/clone_shim.rs:97, :683-705). `ty_contains_unresolvable` recurses into `FuncThick` params, so `f: escaping (I.A) -> Bool` counts as "unresolvable" because of `AssociatedProjection`. `W` therefore never gets the clone shim or the `type_info.copy = Clone(_)` overwrite that escaping-closure fields require (`ty_needs_clone_shim`, :720-732). `W[S]` stays `Bitwise`, and `verify.rs:175-205` (`bad_closure_field`) ICEs. A field of `escaping (Int64) -> Bool` or `escaping (I) -> Bool` is fine; only an assoc projection inside the closure signature trips it.
**Confidence:** high.
**Fix shape:** don't treat a `FuncThick`'s param/ret types as unresolvable (its clone is a handle retain independent of the signature), or run the skip check after per-instantiation assoc resolution.

## H2 (generic part): nested struct or enum inside a struct

**Root cause.** `lower_types` (lib/kestrel-mir-lower/src/items/mod.rs:50-63) recurses only into `Module` children. A `Struct` or `Enum` declared inside a struct is never lowered to a MIR StructDef/EnumDef. `lower_member_functions` (:93) also never descends into nested types. Mono sees `Named(Inner, [])` without a MonoStruct, and `verify.rs:712` ICEs. This reproduces identically for a non-generic `Outer`, so genericity is incidental.
**Confidence:** high.
**Fix shape:** recurse into `Struct`/`Enum` children in `lower_types` and `lower_member_functions` (and anything else that walks types).

## M13: nested generic loses the outer type parameter

**Root cause.** A nested `Inner[U]` that mentions the outer `T` has `TypeParams = [U]` only. Neither the HIR type `Inner[String]` nor the solver's member substitution (lib/kestrel-type-infer/src/solver/member.rs:~800-812, `struct_type_params.zip(recv_type_args)`) carries the enclosing `T`. `i.a : T` stays a rigid outer param at the use site, so `no member 'notEqual' on type 'T'`. Even after that is fixed, MIR (H2) never lowers the nested type.
**Confidence:** medium-high.
**Fix shape:** model a nested type's effective params as outer params ++ own params (implicitly threaded at HIR lowering and in `fresh_type_args`), or reject nested types that capture outer params until that exists.

## M12: `Box.make(3)` on an unspecialized generic type leaks the rigid `T`

**Root cause.** `instantiate_entity_inner` (lib/kestrel-type-infer/src/generate.rs:2402) creates fresh TyVars for the parent's type params only when the parent is an `Extension` (:2506-2530, "For methods/functions inside extensions…"). The parent-param mapping at :2471 runs only with explicit type args. For a static method declared directly in `struct Box[T]`, `Box.make` (a Def of `make`) has no own params and no fresh `T`, so the signature keeps the rigid `Param(T)` (`T !: ExpressibleByIntLiteral`). `Box[Int64].make(3)` works.
**Confidence:** high.
**Fix shape:** extend the no-explicit-args freshening to a parent that is a `Struct`/`Enum` (substituting the parent's `TypeParams` with fresh vars), mirroring the extension branch.

## M14: `T.A = U.A` doesn't carry the `T.A: Equatable` bound to `U.A`

**Root cause.** Conformance of a projection is filtered by receiver: `assoc_projection_base_admits` (resolve.rs:870-923) admits a `Bound` clause only when its subject spine equals the query's spine (`[T,A]` vs `[U,A]`). The solver unified `T.A` and `U.A` (the error names `U.A`), but the same-type where clause `T.A = U.A` is never consulted. `x.a() == y.a()` dispatches on `U.A`, which has no Equatable bound, and fails.
**Confidence:** medium-high.
**Fix shape:** have `assoc_projection_base_admits` and the projection member lookup accept spines in the same equivalence class under the owner's same-type clauses (or canonicalize projections to a representative before the spine compare).

## M15: generic tuple alias `type Pair[T] = (T, T)` is not expanded

**Root cause.** HIR lowering eagerly expands only trivial, non-generic aliases (lib/kestrel-hir-lower/src/ty.rs:288-296). `Pair[T]` stays `HirTy::AliasUse` → `TyKind::TypeAlias`. The solver reduces aliases only in `solve_equal` (lib/kestrel-type-infer/src/solver/equality.rs:6-42). `solve_coerce` (return, `let` annotation, args), tuple indexing and member lookup see the unreduced `Pair[T]` vs `(T,T)`, which gives a mismatch, or "cannot index into non-tuple type 'Pair[Int64]'".
**Confidence:** medium-high.
**Fix shape:** expand bound-free generic aliases at HIR lowering by substituting args into the annotation, or reduce `TypeAlias` before coerce, projection and member resolution.

## M16: `extend [T]: P` array sugar is not resolved

**Root cause.** `ExtensionTargetEntity` (lib/kestrel-name-res/src/extensions.rs:50-63) handles `Unit`/`Never`/`Ref`/`Named` only. `AstType::Array` returns `None`. `LowerExtensionTargetTypeArgs` (lib/kestrel-hir-lower/src/ty.rs:1291) likewise returns `[]` for non-Named targets. The extension targets nothing, so `self` and the element are untyped (E106) and `[Int64]: P` doesn't hold. Even `extend [T] { func foo() }` fails with "no member 'foo' on Array". `ResolvedExtensionTarget` is never inserted anywhere.
**Confidence:** high.
**Fix shape:** desugar `AstType::Array`/`Dictionary`/`Optional` extension targets to their `std` nominal entity plus element args in both queries (ideally once, at AST build or name-res).

## H22: polymorphic recursion hangs

**Root cause.** Each `recur[T]` instantiation requests `recur[Array[T]]`. The mono collector's BFS (lib/kestrel-mir/src/mono/collect.rs:199, `while let Some(key) = self.queue.pop_front()`) has no depth or type-size bound, and the front end has no polymorphic-recursion check. `dump mir -s raw` finishes fine, so the hang is in mono.
**Confidence:** high.
**Fix shape:** cap the instantiation type depth or size in `collect` and emit a user diagnostic ("polymorphic recursion / infinite instantiation"), ideally anchored at the recursive call.
