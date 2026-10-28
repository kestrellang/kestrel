# Implementation Plans

> **Historical design documents.** These are the plans that were written *before*
> (or while) each feature was built. They are point-in-time snapshots — the
> shipped implementation often diverged. Do not use them as references for
> current compiler behavior; see [docs/language/](../language/) and
> [docs/memory-model/](../memory-model/) for that. The Status column reflects
> whether the *feature* is in the compiler today, not whether the plan was
> followed.

| Feature | Status | Plans |
|---------|--------|-------|
| [Array Literals](array-literals/) | Shipped | array-literals |
| [Array Matchable](array-matchable/) | Shipped | array-matchable-design, array-matchable-plan |
| [Boolean Guard](boolean-guard.md) | Shipped | boolean-guard |
| [Char Literals](char-literals/) | Shipped | char-literals-design, char-literals-plan |
| [Closures](closures/) | Shipped | closure-plan, trailing-closures-plan, thunk-generation-plan |
| [Compound Assignment](compound-assignment/) | Shipped | compound-assignment-design, compound-assignment-plan |
| [Computed Properties](computed-properties/) | Shipped | computed-properties-plan |
| [Conformance Specialization](conformance-specialization.md) | Shipped | conformance-specialization |
| [Default Function Parameters](default-function-parameters/) | Shipped | default-function-parameters-design, default-function-parameters-plan |
| [Dictionary Literals](dictionary-literals/) | Shipped | dictionary-literals-design, dictionary-literals-plan |
| [Drop Shim Activation](drop-shim-activation/) | Shipped (empty dir — plan never written; drop shims exist in MIR) | — |
| [Enums](enums/) | Shipped | enum-plan, enum-new-plan |
| [Execution Graph](execution-graph/) | Superseded (original MIR plan; MIR has since been rewritten — see [docs/refactor/](../refactor/)) | execution-graph-implementation |
| [Exitable `@main` Returns](exitable-main-return.md) | Shipped | exitable-main-return |
| [Expression-Bodied Functions](expression-bodied-functions/) | Shipped | expression-bodied-functions-design, expression-bodied-functions-plan |
| [Extensions](extensions/) | Shipped | protocol-extensions, extensions-implementation-plan, protocol-extensions-implementation-plan |
| [For Loops](for-loops/) | Shipped | for-loops-design, for-loops-plan |
| [Generics](generics/) | Shipped | generic-protocol-bounds |
| [Indirection (indirect enums)](indirection/) | Not shipped (the analyzer still rejects `indirect` enums as unsupported) | README, syntax, semantics, compiler-arch, errors, tests |
| [Keywords as Labels](keywords-as-labels.md) | Shipped | keywords-as-labels |
| [Lang Ptr](lang-ptr/) | Shipped | lang-ptr-implementation |
| [Memory Model](memory-model/) | Superseded (current spec lives in [docs/memory-model/](../memory-model/)) | memory-model-ideas |
| [Monomorphization Witness Fix](monomorphization-witness-fix-plan.md) | Shipped (unverified — believed resolved by later witness work) | monomorphization-witness-fix-plan |
| [Null Coalescing](null-coalescing/) | Shipped | null-coalescing-design, null-coalescing-plan |
| [Opaque Types (`some P`)](opaque-types/) | Shipped | design, implementation-spec |
| [Operators](operators/) | Shipped | operator-protocols |
| [Optional/Result Promotion](optional-result-promotion/) | Shipped | optional-result-promotion-design, optional-result-promotion-plan, optional-result-promotion-plan-v2 |
| [Optional/Throwing Inits](optional-throwing-inits.md) | Shipped | optional-throwing-inits |
| [Partial Ranges](partial-ranges.md) | Shipped | partial-ranges |
| [Pattern Matching](pattern-matching/) | Shipped | pattern-matching-plan |
| [Range Matchable](range-matchable/) | Shipped | range-matchable-design, range-matchable-plan |
| [References (`&T`)](references/) | Shipped (2026-06; closures/capture stage still in progress) | README, stage0.5 → stage3 |
| [Short Circuit (`and`/`or`)](short-circuit/) | Shipped | short-circuit-design, short-circuit-plan |
| [Static Properties](static-properties/) | Shipped | static-properties-design, static-properties-plan |
| [String Interpolation](string-interpolation/) | Shipped | string-interpolation-design, string-interpolation-plan |
| [Subscripts](subscripts/) | Shipped | subscripts-plan |
| [Throw Expression](throw_expression/) | Shipped | throw_expression-design, throw_expression-plan |
| [Try Operator](try-operator/) | Shipped | try-operator |
| [Type Operators](type-operators/) | Shipped | type-operators-design, type-operators-plan |
