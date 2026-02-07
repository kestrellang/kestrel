# Static Properties Implementation Plan

## Overview

This plan implements static properties as designed in `static-properties-design.md`. The work is broken into phases following the compiler pipeline.

## Test Strategy

The tests already exist in `lib/kestrel-test-suite/tests/validation/properties_intended.rs` (29 failing, 3 passing). Implementation is complete when all 32 tests pass.

**Test categories:**
- Global properties (module-level let/var)
- Struct static properties (stored and computed)
- Enum static properties (stored and computed, instance stored banned)
- Protocol static property requirements and conformance

## Already Implemented

- **`newValue` binding**: Setter binder already creates `newValue` parameter and binds it to local scope
- **Getter/Setter signatures**: CallableBehavior with correct types and receiver kinds
- **Field `is_static` flag**: Parser and builder correctly track static modifier
- **Static-aware receiver**: Getters/setters correctly have no receiver when static

## Implementation Phases

### Phase 1: Validation Analyzers (Quick Wins)

Add analyzers for error cases that should fail compilation. These are straightforward checks.

**Files:**
- `lib/kestrel-semantic-analyzers/src/analyzers/field/mod.rs` (new or extend existing)
- `lib/kestrel-semantic-analyzers/src/lib.rs` (register analyzer)

**Tasks:**
- [ ] 1.1: "computed properties must use 'var'" - error when `let` has accessor block
- [ ] 1.2: "properties in global context are already static" - error when module-level field has `static`
- [ ] 1.3: "enums cannot have stored fields" - error for non-static, non-computed field in enum
- [ ] 1.4: "static stored properties not supported in generic types" - error for static stored in generic struct/enum

**Tests covered:** `*_disallowed` tests (approximately 10 tests)

---

### Phase 2: ~~Computed Property Setters (`newValue` binding)~~ DONE

✅ Already implemented in `lib/kestrel-semantic-tree-binder/src/binders/setter.rs`:
- Lines 59-66: Creates `newValue` parameter with field type
- Lines 161-173: Binds `newValue` to local scope during body resolution

---

### Phase 3: Computed Property Access (Read)

Make reading computed properties call the getter.

**Files:**
- `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`
- `lib/kestrel-semantic-tree/src/behavior/computed_member_access.rs`

**Tasks:**
- [ ] 3.1: When accessing a computed property, resolve to getter call
- [ ] 3.2: Ensure `ComputedMemberAccessBehavior` is created for computed fields
- [ ] 3.3: Handle both instance (`self.prop`) and static (`Type.prop`) computed reads

**Tests covered:** Read side of `*_get_set` tests

---

### Phase 4: Computed Property Access (Write)

Make assigning to computed properties call the setter.

**Files:**
- `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` (assignment handling)
- `lib/kestrel-execution-graph-lowering/src/` (MIR lowering for setter calls)

**Tasks:**
- [ ] 4.1: Detect assignment to computed property in body resolver
- [ ] 4.2: Transform assignment into setter call with RHS as argument
- [ ] 4.3: Handle compound assignment (`+=`, etc.) as get-modify-set sequence

**Tests covered:** Write side of `*_get_set` tests

---

### Phase 5: Static Property Access via Type Name

Enable `Type.staticProp` syntax for accessing static properties.

**Files:**
- `lib/kestrel-semantic-tree-binder/src/body_resolver/paths.rs`
- `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

**Tasks:**
- [ ] 5.1: When path resolves to a type, allow `.member` access for static members
- [ ] 5.2: Create `TypeRef` expression for the base, then field access for the property
- [ ] 5.3: Ensure static stored properties return correct `MemberAccessBehavior`
- [ ] 5.4: Ensure static computed properties return correct `ComputedMemberAccessBehavior`

**Tests covered:** `struct_static_let_initial_value`, `struct_static_var_mutability_and_initial_value`, `enum_static_*`

---

### Phase 6: Name Resolution in Static Context

Enable unqualified access to static members from within static context.

**Files:**
- `lib/kestrel-semantic-tree-binder/src/body_resolver/paths.rs`

**Tasks:**
- [ ] 6.1: Detect when body resolver is in static context (static method, static computed property)
- [ ] 6.2: When resolving unqualified name, check static members of enclosing type
- [ ] 6.3: Error if `self` is used in static context

**Tests covered:** Static computed property tests that access `_s` without qualification

---

### Phase 7: Static Property Storage (Codegen)

Allocate global storage for static stored properties.

**Current issue:** MIR lowering fails with "type reference as value" and "field access on immediate value" when accessing `Type.staticProp`.

**Files:**
- `lib/kestrel-execution-graph/src/function/place.rs` - Add `PlaceKind::Static`
- `lib/kestrel-execution-graph-lowering/src/` - Handle static property lowering
- `lib/kestrel-codegen-cranelift/src/context.rs` - Allocate DataId for statics
- `lib/kestrel-codegen-cranelift/src/place.rs` - Compile static place access
- `lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs` - Collect static properties

**Tasks:**
- [ ] 7.1: Add `PlaceKind::Static { symbol: SymbolId, name: String }` to execution graph
- [ ] 7.2: In MIR lowering, convert `TypeRef` + field access to `PlaceKind::Static`
- [ ] 7.3: Collect all static stored properties during monomorphization
- [ ] 7.4: Allocate `DataId` for each static property (similar to string literals)
- [ ] 7.5: For constant initializers, embed value directly in data section
- [ ] 7.6: Generate `__kestrel_init_statics()` for complex initializers
- [ ] 7.7: Call `__kestrel_init_statics()` at program entry before `main()`

**Tests covered:** All static stored property tests

---

### Phase 8: Global (Module-Level) Properties

Enable module-level `let`/`var` declarations.

**Files:**
- `lib/kestrel-semantic-tree-builder/src/builders/field.rs`
- `lib/kestrel-semantic-tree-binder/src/body_resolver/paths.rs`

**Tasks:**
- [ ] 8.1: Ensure module-level fields are built correctly (already partially works)
- [ ] 8.2: Resolve unqualified access to module-level properties
- [ ] 8.3: Handle module-level computed properties (get/set at module scope)

**Tests covered:** `global_let_initial_value`, `global_var_mutability_and_initial_value`, `global_computed_var_get_set`

---

### Phase 9: Protocol Static Property Conformance

Check that types correctly implement protocol static property requirements.

**Files:**
- `lib/kestrel-semantic-model/src/queries/protocol_required_properties.rs`
- `lib/kestrel-semantic-analyzers/src/analyzers/` (conformance checking)

**Tasks:**
- [ ] 9.1: Include static properties in protocol requirement collection
- [ ] 9.2: Check property type matches exactly
- [ ] 9.3: Check mutability satisfaction (see design doc table)
- [ ] 9.4: Emit "property 'X' has wrong type for protocol" error on mismatch

**Tests covered:** All `protocol_*_type_mismatch` tests

---

### Phase 10: Witness Table for Static Properties

Enable `T.staticProp` where T is a type parameter bounded by protocol.

**Files:**
- `lib/kestrel-codegen-cranelift/src/monomorphize/witness.rs`
- `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

**Tasks:**
- [ ] 10.1: Add witness table entries for static properties (getter/setter pointers)
- [ ] 10.2: When accessing `T.staticProp` with generic T, go through witness table
- [ ] 10.3: Generate witness implementations for conforming types

**Note:** This phase may be deferred if no tests require it currently.

---

## Suggested Implementation Order

Based on dependencies, test coverage, and current state:

1. **Phase 1** (Validation) - Quick wins, enables ~10 error tests
2. ~~Phase 2~~ - Already done
3. **Phase 4** (Computed write) - Fix "cannot assign to immutable field" for computed vars
4. **Phase 3** (Computed read) - Ensure getter calls work (may already work partially)
5. **Phase 7** (Codegen storage) - Add `PlaceKind::Static`, fix MIR lowering
6. **Phase 5** (Type.staticProp) - Depends on Phase 7
7. **Phase 6** (Static context resolution) - Needed for static computed tests
8. **Phase 8** (Global properties) - Module-level tests
9. **Phase 9** (Protocol conformance) - Protocol tests
10. **Phase 10** (Witness tables) - Deferred unless needed

**Critical path:** Phase 4 → Phase 7 → Phase 5 (these unlock the most tests)

## Verification

After each phase:
```bash
cargo test -p kestrel-test-suite properties_intended
cargo clippy
cargo fmt --check
```

Final verification:
```bash
cargo test  # All tests pass
```

## Risk Areas

1. **Assignment detection** - Need to intercept assignments to computed properties at the right level
2. **Static context tracking** - Must propagate through nested contexts correctly
3. **Initialization ordering** - Complex initializers need careful ordering
4. **MIR representation** - May need new expression kinds for static property access

## Out of Scope (Deferred)

- Thread safety for static var mutation
- Lazy initialization
- Cross-module static reference detection in initializers (warning only for now)
